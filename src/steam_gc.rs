use std::{
    ffi::{c_char, c_void},
    fs,
    os::windows::ffi::OsStrExt,
    path::Path,
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use libloading::Library;
use windows_sys::Win32::{
    Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
    System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    },
    System::LibraryLoader::SetDllDirectoryW,
};

use crate::protobuf::{PROTO_MASK, body_from_packet, proto_packet, tf_client_init};

const GC_CLIENT_WELCOME: u32 = 4004;
const GC_CLIENT_HELLO: u32 = 4006;
const MATCHMAKER_STATS_REQUEST: u32 = 6524;
const MATCHMAKER_STATS_RESPONSE: u32 = 6525;
const TF_CLIENT_INIT: u32 = 6536;

type SteamApiInit = unsafe extern "C" fn() -> bool;
type SteamApiShutdown = unsafe extern "C" fn();
type SteamApiRunCallbacks = unsafe extern "C" fn();
type SteamApiGetHSteamUser = unsafe extern "C" fn() -> i32;
type SteamFindUserInterface = unsafe extern "C" fn(i32, *const c_char) -> *mut c_void;
type GcSendMessage = unsafe extern "system" fn(*mut c_void, u32, *const c_void, u32) -> i32;
type GcIsMessageAvailable = unsafe extern "system" fn(*mut c_void, *mut u32) -> bool;
type GcRetrieveMessage =
    unsafe extern "system" fn(*mut c_void, *mut u32, *mut c_void, u32, *mut u32) -> i32;

struct SteamApi {
    _library: Library,
    initialized: bool,
    shutdown: SteamApiShutdown,
    run_callbacks: SteamApiRunCallbacks,
    get_user: SteamApiGetHSteamUser,
    find_user_interface: SteamFindUserInterface,
}

impl Drop for SteamApi {
    fn drop(&mut self) {
        if self.initialized {
            // SAFETY: Function pointer came from the loaded Steam API library,
            // retained in this struct until after Drop completes.
            unsafe { (self.shutdown)() };
        }
    }
}

struct GameCoordinator {
    instance: *mut c_void,
    send: GcSendMessage,
    available: GcIsMessageAvailable,
    retrieve: GcRetrieveMessage,
}

fn symbol<T: Copy>(library: &Library, name: &[u8]) -> Result<T> {
    // SAFETY: Callers provide the exact ABI/signature exported by steam_api64.dll.
    unsafe { library.get::<T>(name) }
        .map(|value| *value)
        .with_context(|| format!("Steam export {} is missing", String::from_utf8_lossy(name)))
}

fn load_api(tf2_root: &Path) -> Result<(SteamApi, SteamApiInit)> {
    let bin = tf2_root.join("bin").join("x64");
    let dll = bin.join("steam_api64.dll");
    // These process-global settings are deliberately left in place: this is a
    // short-lived, single-purpose CLI that exits immediately after one query.
    // SAFETY: Modifying this process's environment occurs before worker threads
    // are started and is required for SteamAPI to select app 440.
    unsafe {
        std::env::set_var("SteamAppId", "440");
        std::env::set_var("SteamGameId", "440");
    }
    let mut wide_bin = bin.as_os_str().encode_wide().collect::<Vec<_>>();
    wide_bin.push(0);
    // SAFETY: The path is NUL-terminated and remains alive for the call. This
    // mirrors the lookup behavior used by TF2's own loader for sibling DLLs.
    // It intentionally remains set until this short-lived process exits.
    if unsafe { SetDllDirectoryW(wide_bin.as_ptr()) } == 0 {
        return Err(std::io::Error::last_os_error())
            .context("could not set the TF2 DLL search directory");
    }

    // SAFETY: The path was validated as the installed TF2 Steam API library.
    let library = unsafe { Library::new(&dll) }
        .with_context(|| format!("could not load {}", dll.display()))?;
    let init = symbol::<SteamApiInit>(&library, b"SteamAPI_Init\0")?;
    let api = SteamApi {
        initialized: false,
        shutdown: symbol(&library, b"SteamAPI_Shutdown\0")?,
        run_callbacks: symbol(&library, b"SteamAPI_RunCallbacks\0")?,
        get_user: symbol(&library, b"SteamAPI_GetHSteamUser\0")?,
        find_user_interface: symbol(&library, b"SteamInternal_FindOrCreateUserInterface\0")?,
        _library: library,
    };
    Ok((api, init))
}

fn load_gc(api: &SteamApi) -> Result<GameCoordinator> {
    // SAFETY: Steam API was initialized and function pointers are valid.
    let user = unsafe { (api.get_user)() };
    if user == 0 {
        bail!("Steam returned HSteamUser 0");
    }
    // SAFETY: The interface name is NUL-terminated and the user handle is from Steam.
    let instance = unsafe { (api.find_user_interface)(user, c"SteamGameCoordinator001".as_ptr()) };
    if instance.is_null() {
        bail!("Steam did not expose SteamGameCoordinator001 for app 440");
    }
    // SAFETY: SteamGameCoordinator001 is a C++ interface whose first object member
    // is a vtable with SendMessage, IsMessageAvailable, RetrieveMessage.
    let table = unsafe { *(instance.cast::<*mut *mut c_void>()) };
    if table.is_null() {
        bail!("SteamGameCoordinator001 returned an invalid vtable");
    }
    // SAFETY: These indices and signatures are the documented interface layout.
    let (send, available, retrieve) = unsafe {
        (
            std::mem::transmute::<*mut c_void, GcSendMessage>(*table.add(0)),
            std::mem::transmute::<*mut c_void, GcIsMessageAvailable>(*table.add(1)),
            std::mem::transmute::<*mut c_void, GcRetrieveMessage>(*table.add(2)),
        )
    };
    Ok(GameCoordinator {
        instance,
        send,
        available,
        retrieve,
    })
}

fn send(gc: &GameCoordinator, message_type: u32, body: &[u8]) -> Result<()> {
    let packet = proto_packet(message_type, body);
    // SAFETY: GC instance and vtable are valid; packet remains alive for the call.
    let result = unsafe {
        (gc.send)(
            gc.instance,
            message_type | PROTO_MASK,
            packet.as_ptr().cast(),
            u32::try_from(packet.len()).context("GC packet is too large")?,
        )
    };
    match result {
        0 => Ok(()),
        3 => bail!("Steam is not logged on"),
        _ => bail!("Steam GC SendMessage({message_type}) failed with result {result}"),
    }
}

fn receive(gc: &GameCoordinator) -> Result<Option<(u32, Vec<u8>)>> {
    let mut size = 0_u32;
    // SAFETY: GC instance and output pointer are valid.
    if !unsafe { (gc.available)(gc.instance, &raw mut size) } {
        return Ok(None);
    }
    let mut packet = vec![0_u8; usize::try_from(size.max(8)).expect("u32 fits usize")];
    let mut raw_type = 0_u32;
    let mut received = 0_u32;
    // SAFETY: Buffer and output pointers are valid for the duration of the call.
    let mut result = unsafe {
        (gc.retrieve)(
            gc.instance,
            &raw mut raw_type,
            packet.as_mut_ptr().cast(),
            u32::try_from(packet.len()).expect("GC buffer came from u32"),
            &raw mut received,
        )
    };
    if result == 2 {
        packet.resize(usize::try_from(received).expect("u32 fits usize"), 0);
        // SAFETY: Same as above, now using the requested buffer size.
        result = unsafe {
            (gc.retrieve)(
                gc.instance,
                &raw mut raw_type,
                packet.as_mut_ptr().cast(),
                u32::try_from(packet.len()).expect("GC buffer came from u32"),
                &raw mut received,
            )
        };
    }
    if result != 0 {
        bail!("Steam GC RetrieveMessage failed with result {result}");
    }
    packet.truncate(usize::try_from(received).expect("u32 fits usize"));
    Ok(Some((raw_type, packet)))
}

fn client_version(tf2_root: &Path) -> Result<u32> {
    let steam_inf = fs::read_to_string(tf2_root.join("tf").join("steam.inf"))
        .context("could not read tf/steam.inf")?;
    steam_inf
        .lines()
        .find_map(|line| line.strip_prefix("ClientVersion="))
        .context("tf/steam.inf has no ClientVersion")?
        .trim()
        .parse()
        .context("ClientVersion is not an unsigned integer")
}

pub fn tf2_is_running() -> bool {
    // SAFETY: Snapshot handle is closed below, and PROCESSENTRY32W is initialized as
    // required by ToolHelp.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return false;
        }
        let mut entry = PROCESSENTRY32W {
            dwSize: u32::try_from(std::mem::size_of::<PROCESSENTRY32W>())
                .expect("structure size fits u32"),
            ..std::mem::zeroed()
        };
        let mut found = false;
        if Process32FirstW(snapshot, &raw mut entry) != 0 {
            loop {
                let length = entry
                    .szExeFile
                    .iter()
                    .position(|character| *character == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..length]);
                if name.eq_ignore_ascii_case("tf_win64.exe") {
                    found = true;
                    break;
                }
                if Process32NextW(snapshot, &raw mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snapshot);
        found
    }
}

pub fn request_map_counts(tf2_root: &Path, timeout: Duration) -> Result<Vec<u32>> {
    let version = client_version(tf2_root)?;
    let (mut api, init) = load_api(tf2_root)?;
    // SAFETY: Function pointer is valid while `api` retains the library.
    if !unsafe { init() } {
        bail!("SteamAPI_Init failed; ensure desktop Steam is running and logged in");
    }
    api.initialized = true;
    let gc = load_gc(&api)?;
    let started = Instant::now();
    let mut last_hello = started
        .checked_sub(Duration::from_secs(3))
        .unwrap_or(started);
    let mut welcomed = false;
    let mut init_sent = false;
    let mut request_sent = false;

    while started.elapsed() < timeout {
        // SAFETY: Steam API remains initialized and loaded.
        unsafe { (api.run_callbacks)() };
        if !welcomed && last_hello.elapsed() >= Duration::from_millis(2_500) {
            send(&gc, GC_CLIENT_HELLO, &[])?;
            last_hello = Instant::now();
        }
        while let Some((raw_type, packet)) = receive(&gc)? {
            let message_type = raw_type & !PROTO_MASK;
            if message_type == GC_CLIENT_WELCOME {
                welcomed = true;
            } else if message_type == MATCHMAKER_STATS_RESPONSE {
                return crate::protobuf::decode_map_counts(body_from_packet(raw_type, &packet));
            }
        }
        if welcomed && !init_sent {
            send(&gc, TF_CLIENT_INIT, &tf_client_init(version))?;
            init_sent = true;
        }
        if init_sent && !request_sent {
            send(&gc, MATCHMAKER_STATS_REQUEST, &[])?;
            request_sent = true;
        }
        thread::sleep(Duration::from_millis(20));
    }
    if welcomed {
        bail!("timed out waiting for TF2 GC message 6525 after passive request")
    }
    bail!("timed out waiting for the TF2 Game Coordinator welcome")
}

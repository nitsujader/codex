use std::ffi::OsStr;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;

use windows::UI::Notifications::ToastNotification;
use windows::UI::Notifications::ToastNotificationManager;
use windows::Win32::System::Com::CLSCTX_INPROC_SERVER;
use windows::Win32::System::Com::COINIT_APARTMENTTHREADED;
use windows::Win32::System::Com::CoCreateInstance;
use windows::Win32::System::Com::CoInitializeEx;
use windows::Win32::System::Com::CoTaskMemAlloc;
use windows::Win32::System::Com::CoUninitialize;
use windows::Win32::System::Com::IPersistFile;
use windows::Win32::System::Variant::VT_LPWSTR;
use windows::Win32::UI::Shell::IShellLinkW;
use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;
use windows::Win32::UI::Shell::PropertiesSystem::PROPERTYKEY;
use windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;
use windows::Win32::UI::Shell::ShellLink;
use windows::core::GUID;
use windows::core::HSTRING;
use windows::core::Interface;
use windows::core::PCWSTR;
use windows::core::PROPVARIANT;

use super::bel::BelBackend;

const AUMID: &str = "com.openai.codex";
const SHORTCUT_FILENAME: &str = "Codex.lnk";
const TOAST_TITLE: &str = "Codex";

const PKEY_APP_USER_MODEL_ID: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID::from_u128(0x9f4c2855_9f79_4b39_a8d0_e1d42de1d5f3),
    pid: 5,
};

#[derive(Debug, Default)]
pub struct WindowsToastBackend {
    bel: BelBackend,
    disabled: bool,
}

impl WindowsToastBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn notify(&mut self, message: &str) -> io::Result<()> {
        if self.disabled {
            return self.bel.notify(message);
        }

        if let Err(err) = ensure_toast_registered().and_then(|()| show_toast(message)) {
            tracing::warn!("windows toast notifications failed; falling back to BEL: {err}");
            self.disabled = true;
            return self.bel.notify(message);
        }

        Ok(())
    }
}

static REGISTRATION: OnceLock<Result<(), String>> = OnceLock::new();

fn ensure_toast_registered() -> io::Result<()> {
    match REGISTRATION.get_or_init(|| init_toast_registration().map_err(|err| err.to_string())) {
        Ok(()) => Ok(()),
        Err(err) => Err(io::Error::other(err.clone())),
    }
}

fn init_toast_registration() -> io::Result<()> {
    let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    if hr.is_err() {
        return Err(io::Error::other(format!("CoInitializeEx failed: {hr:?}")));
    }

    let result = register_aumid();
    unsafe {
        CoUninitialize();
    }
    result
}

fn register_aumid() -> io::Result<()> {
    let exe_path = std::env::current_exe()?;
    let shortcut_path = start_menu_shortcut_path()?;
    create_shortcut_with_aumid(shortcut_path.as_path(), exe_path.as_path(), AUMID)?;

    let aumid_w = to_wide(OsStr::new(AUMID));
    unsafe {
        SetCurrentProcessExplicitAppUserModelID(PCWSTR(aumid_w.as_ptr())).map_err(|err| {
            io::Error::other(format!(
                "SetCurrentProcessExplicitAppUserModelID failed: {err:?}"
            ))
        })?;
    }

    Ok(())
}

fn show_toast(message: &str) -> io::Result<()> {
    let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    if hr.is_err() {
        return Err(io::Error::other(format!("CoInitializeEx failed: {hr:?}")));
    }

    let result = show_toast_impl(message);
    unsafe {
        CoUninitialize();
    }
    result
}

fn show_toast_impl(message: &str) -> io::Result<()> {
    let escaped = escape_xml_text(message);
    let xml = format!(
        "<toast><visual><binding template=\"ToastGeneric\"><text>{TOAST_TITLE}</text><text>{escaped}</text></binding></visual></toast>"
    );

    let doc = windows::Data::Xml::Dom::XmlDocument::new()
        .map_err(|err| io::Error::other(format!("XmlDocument::new failed: {err:?}")))?;
    doc.LoadXml(&HSTRING::from(xml))
        .map_err(|err| io::Error::other(format!("XmlDocument::LoadXml failed: {err:?}")))?;

    let toast = ToastNotification::CreateToastNotification(&doc)
        .map_err(|err| io::Error::other(format!("ToastNotification create failed: {err:?}")))?;

    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(AUMID))
        .map_err(|err| io::Error::other(format!("CreateToastNotifierWithId failed: {err:?}")))?;
    notifier
        .Show(&toast)
        .map_err(|err| io::Error::other(format!("ToastNotifier::Show failed: {err:?}")))?;

    Ok(())
}

fn escape_xml_text(input: &str) -> String {
    // Only escape what is needed for XML element text content.
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
    out
}

fn start_menu_shortcut_path() -> io::Result<PathBuf> {
    let Some(appdata) = dirs::data_dir() else {
        return Err(io::Error::other(
            "failed to resolve AppData/Roaming directory",
        ));
    };
    let dir = appdata
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs");
    std::fs::create_dir_all(dir.as_path())?;
    Ok(dir.join(SHORTCUT_FILENAME))
}

fn create_shortcut_with_aumid(
    shortcut_path: &Path,
    target_exe: &Path,
    aumid: &str,
) -> io::Result<()> {
    let shell_link: IShellLinkW = unsafe {
        CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
    }
    .map_err(|err| io::Error::other(format!("CoCreateInstance ShellLink failed: {err:?}")))?;

    let target_w = to_wide(target_exe.as_os_str());
    unsafe {
        shell_link
            .SetPath(PCWSTR(target_w.as_ptr()))
            .map_err(|err| io::Error::other(format!("IShellLinkW::SetPath failed: {err:?}")))?;
    }

    let workdir = target_exe.parent().unwrap_or_else(|| Path::new("."));
    let workdir_w = to_wide(workdir.as_os_str());
    unsafe {
        shell_link
            .SetWorkingDirectory(PCWSTR(workdir_w.as_ptr()))
            .map_err(|err| {
                io::Error::other(format!("IShellLinkW::SetWorkingDirectory failed: {err:?}"))
            })?;
    }

    let desc_w = to_wide(OsStr::new("Codex"));
    unsafe {
        shell_link
            .SetDescription(PCWSTR(desc_w.as_ptr()))
            .map_err(|err| {
                io::Error::other(format!("IShellLinkW::SetDescription failed: {err:?}"))
            })?;
    }

    let store: IPropertyStore = shell_link
        .cast()
        .map_err(|err| io::Error::other(format!("cast IPropertyStore failed: {err:?}")))?;
    let pv = propvariant_lpwstr(aumid)?;
    unsafe {
        store
            .SetValue(&PKEY_APP_USER_MODEL_ID, &pv)
            .map_err(|err| io::Error::other(format!("IPropertyStore::SetValue failed: {err:?}")))?;
        store
            .Commit()
            .map_err(|err| io::Error::other(format!("IPropertyStore::Commit failed: {err:?}")))?;
    }

    let persist: IPersistFile = shell_link
        .cast()
        .map_err(|err| io::Error::other(format!("cast IPersistFile failed: {err:?}")))?;
    let shortcut_w = to_wide(shortcut_path.as_os_str());
    unsafe {
        persist
            .Save(PCWSTR(shortcut_w.as_ptr()), true)
            .map_err(|err| io::Error::other(format!("IPersistFile::Save failed: {err:?}")))?;
    }

    Ok(())
}

fn propvariant_lpwstr(value: &str) -> io::Result<PROPVARIANT> {
    let wide = to_wide(OsStr::new(value));
    let bytes = wide.len().saturating_mul(std::mem::size_of::<u16>());

    let raw_ptr = unsafe { CoTaskMemAlloc(bytes) }.cast::<u16>();
    if raw_ptr.is_null() {
        return Err(io::Error::other("CoTaskMemAlloc failed"));
    }

    unsafe {
        std::ptr::copy_nonoverlapping(wide.as_ptr(), raw_ptr, wide.len());
    }

    let raw = windows::core::imp::PROPVARIANT {
        Anonymous: windows::core::imp::PROPVARIANT_0 {
            Anonymous: windows::core::imp::PROPVARIANT_0_0 {
                vt: VT_LPWSTR.0,
                wReserved1: 0,
                wReserved2: 0,
                wReserved3: 0,
                Anonymous: windows::core::imp::PROPVARIANT_0_0_0 { pwszVal: raw_ptr },
            },
        },
    };

    Ok(unsafe { PROPVARIANT::from_raw(raw) })
}

fn to_wide(value: &OsStr) -> Vec<u16> {
    let mut wide: Vec<u16> = value.encode_wide().collect();
    wide.push(0);
    wide
}

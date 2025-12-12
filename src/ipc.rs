use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::config::{self, DesktopLabel};

const PIPE_NAME: &str = r"\\.\pipe\Acme.DesktopLabeler.mddsklbl";

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum Request {
    List,
    ResolveWindow { hwnd: u64 },
}

#[derive(Debug, Serialize)]
struct Response {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    labels: Option<HashMap<String, DesktopLabel>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    desktop_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<DesktopLabel>,
}

pub fn start_server() {
    std::thread::spawn(|| {
        if let Err(e) = run_server_forever() {
            tracing::error!(error=%e, "ipc server stopped");
        }
    });
}

fn run_server_forever() -> anyhow::Result<()> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{ERROR_PIPE_CONNECTED, HANDLE};
    use windows::Win32::Storage::FileSystem::{
        FlushFileBuffers, WriteFile, FILE_FLAGS_AND_ATTRIBUTES, PIPE_ACCESS_DUPLEX,
    };
    use windows::Win32::System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_MESSAGE,
        PIPE_TYPE_MESSAGE, PIPE_WAIT,
    };

    let mut pipe_name: Vec<u16> = PIPE_NAME.encode_utf16().chain(std::iter::once(0)).collect();
    let pipe_name = PCWSTR(pipe_name.as_mut_ptr());

    loop {
        // Create a fresh instance per client.
        let handle = unsafe {
            CreateNamedPipeW(
                pipe_name,
                FILE_FLAGS_AND_ATTRIBUTES(PIPE_ACCESS_DUPLEX.0),
                PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT,
                1,
                64 * 1024,
                64 * 1024,
                0,
                None,
            )
        };

        if handle.is_invalid() {
            let e = windows::core::Error::from_win32();
            tracing::error!(error=%e, "CreateNamedPipeW failed");
            std::thread::sleep(std::time::Duration::from_secs(1));
            continue;
        }

        let handle_guard = HandleGuard(handle);

        let connected = unsafe { ConnectNamedPipe(handle_guard.0, None) };
        if let Err(e) = connected {
            if e.code() != windows::core::HRESULT::from_win32(ERROR_PIPE_CONNECTED.0) {
                tracing::warn!(error=%e, "ConnectNamedPipe failed");
                continue;
            }
        }

        let response = match read_request(handle_guard.0) {
            Ok(Request::List) => match list_labels() {
                Ok(labels) => Response {
                    ok: true,
                    error: None,
                    labels: Some(labels),
                    desktop_id: None,
                    label: None,
                },
                Err(e) => Response {
                    ok: false,
                    error: Some(format!("list failed: {e}")),
                    labels: None,
                    desktop_id: None,
                    label: None,
                },
            },
            Ok(Request::ResolveWindow { hwnd }) => match resolve_window(hwnd) {
                Ok((desktop_id, label)) => Response {
                    ok: true,
                    error: None,
                    labels: None,
                    desktop_id: Some(desktop_id),
                    label: Some(label),
                },
                Err(e) => Response {
                    ok: false,
                    error: Some(format!("resolve_window failed: {e}")),
                    labels: None,
                    desktop_id: None,
                    label: None,
                },
            },
            Err(e) => Response {
                ok: false,
                error: Some(format!("bad request: {e}")),
                labels: None,
                desktop_id: None,
                label: None,
            },
        };

        let payload = serde_json::to_vec(&response).unwrap_or_else(|e| {
            serde_json::to_vec(&Response {
                ok: false,
                error: Some(format!("serialize failed: {e}")),
                labels: None,
                desktop_id: None,
                label: None,
            })
            .expect("serialize minimal error response")
        });

        unsafe {
            let mut written: u32 = 0;
            let _ = WriteFile(
                handle_guard.0,
                Some(&payload),
                Some(&mut written as *mut u32),
                None,
            );
            let _ = FlushFileBuffers(handle_guard.0);
            let _ = DisconnectNamedPipe(handle_guard.0);
        }
    }

    fn read_request(handle: HANDLE) -> anyhow::Result<Request> {
        use windows::Win32::Storage::FileSystem::ReadFile;

        let mut buf = vec![0u8; 64 * 1024];
        let mut read: u32 = 0;
        unsafe {
            ReadFile(
                handle,
                Some(buf.as_mut_slice()),
                Some(&mut read as *mut u32),
                None,
            )?;
        }
        buf.truncate(read as usize);
        Ok(serde_json::from_slice::<Request>(&buf)?)
    }

    fn list_labels() -> anyhow::Result<HashMap<String, DesktopLabel>> {
        let (cfg, _) = config::load_or_default()?;

        let mut out = HashMap::new();
        for (k, v) in cfg.desktops {
            if let Some(guid) = extract_guid_from_key(&k) {
                out.insert(guid.to_string(), v);
            }
        }
        Ok(out)
    }

    fn resolve_window(hwnd: u64) -> anyhow::Result<(String, DesktopLabel)> {
        use core::ffi::c_void;
        use windows::Win32::Foundation::HWND;

        let hwnd = HWND(hwnd as *mut c_void);
        let desktop = winvd::get_desktop_by_window(hwnd)
            .map_err(|e| anyhow::anyhow!(format!("winvd: {e:?}")))?;
        let guid = desktop
            .get_id()
            .map_err(|e| anyhow::anyhow!(format!("winvd: {e:?}")))?;
        let desktop_id = format!("{:?}", guid);

        let (cfg, _) = config::load_or_default()?;
        let key = format!("Desktop(Guid({desktop_id}))");
        let label = cfg.desktops.get(&key).cloned().unwrap_or_default();
        Ok((desktop_id, label))
    }
}

fn extract_guid_from_key(key: &str) -> Option<&str> {
    // Current on-disk format: "Desktop(Guid(<GUID>))"
    let start = key.find("Guid(")? + "Guid(".len();
    let end = key[start..].find(')')? + start;
    Some(&key[start..end])
}

struct HandleGuard(windows::Win32::Foundation::HANDLE);
impl Drop for HandleGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_guid_from_key() {
        assert_eq!(
            extract_guid_from_key("Desktop(Guid(D178F97B-2525-4ED7-B219-6BA2AA6BE296))"),
            Some("D178F97B-2525-4ED7-B219-6BA2AA6BE296")
        );
        assert_eq!(extract_guid_from_key("not a key"), None);
    }
}

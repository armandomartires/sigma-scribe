//! What Settings → Sync shows, and how it is read from Syncthing's answers.

use super::json::Json;

/// Most pending devices or folders listed; the rest wait their turn.
pub const MAX_PENDING: usize = 4;

/// Where the Syncthing process stands.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Run {
    /// Sync is off, or has not been started.
    #[default]
    Off,
    /// The binary is not there.
    NotInstalled,
    /// Started; the API does not answer yet.
    Starting,
    /// The API answers.
    Running,
    /// It could not start or kept stopping: the reason.
    Failed(String),
    /// The loopback network (127.0.0.1) cannot be used, so Syncthing's local
    /// API cannot start. Tried again every few seconds.
    NoLoopback,
}

/// A device the notes folder is shared with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Peer {
    /// Its device ID.
    pub id: String,
    /// The name it was given (the ID's first group when it has none).
    pub name: String,
    /// Connected now.
    pub connected: bool,
}

/// A device that tried to connect and is not accepted yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingDevice {
    /// Its device ID.
    pub id: String,
    /// The name it announced; may be empty.
    pub name: String,
    /// Where it connected from.
    pub address: String,
}

/// A folder a device offers, not accepted yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingFolder {
    /// The folder ID.
    pub id: String,
    /// Its label.
    pub label: String,
    /// The offering device's ID.
    pub device: String,
    /// The offering device's name, when it is a known device.
    pub device_name: String,
}

/// The notes folder as Syncthing reports it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FolderView {
    /// The folder ID.
    pub id: String,
    /// Syncthing's state word: `idle`, `scanning`, `syncing`, `error`, …
    pub state: String,
    /// Files still to fetch.
    pub need_files: u64,
    /// Bytes still to fetch.
    pub need_bytes: u64,
    /// Files that could not be written (`pullErrors`).
    pub errors: u64,
    /// The folder's own error text (`invalid`), when it has one.
    pub error: String,
}

/// Everything the Sync page shows, as the supervisor last read it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct View {
    /// The process.
    pub run: Run,
    /// This device's ID; empty until known.
    pub device_id: String,
    /// The other devices.
    pub peers: Vec<Peer>,
    /// The notes folder, once there is one.
    pub folder: Option<FolderView>,
    /// When a file last arrived, `YYYY-MM-DD HH:MM`.
    pub last_sync: Option<String>,
    /// Devices waiting to be accepted.
    pub pending_devices: Vec<PendingDevice>,
    /// Folders waiting to be accepted.
    pub pending_folders: Vec<PendingFolder>,
    /// Conflict files in the notes folder.
    pub conflicts: usize,
    /// Names of the connected devices that have not accepted the notes folder
    /// (Syncthing's `remoteState` is `notSharing`), in the order of `peers`.
    pub waiting: Vec<String>,
}

/// An empty view, for a model that has no sync to show (tests, the
/// emulator before it is told otherwise).
pub static NO_SYNC: View = View {
    run: Run::Off,
    device_id: String::new(),
    peers: Vec::new(),
    folder: None,
    last_sync: None,
    pending_devices: Vec::new(),
    pending_folders: Vec::new(),
    conflicts: 0,
    waiting: Vec::new(),
};

/// The one line the page shows for the state of things.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// Sync is off.
    Off,
    /// The binary is missing.
    NotInstalled,
    /// Starting.
    Starting,
    /// Stopped, with the reason.
    Failed(String),
    /// The loopback network is down.
    NoLoopback,
    /// Running, and no device is set up yet.
    NoDevices,
    /// Devices are set up and none is connected.
    NotConnected,
    /// A device is connected but no folder is at the notes path, so nothing
    /// can sync (the folder is being made, could not be made, or the
    /// device's offer was ignored).
    NotShared,
    /// Scanning the notes folder.
    Scanning,
    /// A connected device has not accepted the notes folder yet: its name
    /// (the first one), and how many more are waiting too.
    WaitingFor(String, usize),
    /// Fetching: files and bytes still to come.
    Syncing(u64, u64),
    /// Nothing to fetch.
    UpToDate,
    /// The folder reports an error.
    Error(String),
}

impl View {
    /// Devices connected now.
    #[must_use]
    pub fn connected(&self) -> usize {
        self.peers.iter().filter(|p| p.connected).count()
    }

    /// The state of sync as one value.
    #[must_use]
    pub fn status(&self) -> Status {
        match &self.run {
            Run::Off => return Status::Off,
            Run::NotInstalled => return Status::NotInstalled,
            Run::Starting => return Status::Starting,
            Run::Failed(e) => return Status::Failed(e.clone()),
            Run::NoLoopback => return Status::NoLoopback,
            Run::Running => {}
        }
        if let Some(f) = &self.folder {
            if f.state == "error" || !f.error.is_empty() {
                return Status::Error(if f.error.is_empty() {
                    f.state.clone()
                } else {
                    f.error.clone()
                });
            }
            if f.errors > 0 {
                return Status::Error(format!("{} files could not be written", f.errors));
            }
        }
        if self.peers.is_empty() {
            return Status::NoDevices;
        }
        if self.connected() == 0 {
            return Status::NotConnected;
        }
        if self.folder.is_none() {
            return Status::NotShared;
        }
        match self.folder.as_ref() {
            Some(f) if f.state.starts_with("sync") || f.need_files > 0 => {
                Status::Syncing(f.need_files, f.need_bytes)
            }
            // Idle is not "up to date" while a device has yet to accept.
            _ if !self.waiting.is_empty() => {
                Status::WaitingFor(self.waiting[0].clone(), self.waiting.len() - 1)
            }
            Some(f) if f.state.starts_with("scan") => Status::Scanning,
            _ => Status::UpToDate,
        }
    }
}

/// Whether `/rest/db/completion?folder=…&device=…` says the device has not
/// accepted the folder: `remoteState` is `notSharing` (Syncthing 1.20 and
/// later; the other values are `unknown` for a device that is not connected,
/// `paused` and `valid`). An answer without it counts as accepted.
#[must_use]
pub fn not_sharing(completion: &Json) -> bool {
    completion.get("remoteState").and_then(Json::as_str) == Some("notSharing")
}

/// `YYYY-MM-DD HH:MM` from an RFC 3339 time (the part Syncthing writes
/// before the seconds); `None` for the zero time or anything else.
#[must_use]
pub fn short_time(t: &str) -> Option<String> {
    let b = t.as_bytes();
    let ok = b.len() >= 16
        && b[4] == b'-'
        && b[7] == b'-'
        && (b[10] == b'T' || b[10] == b' ')
        && b[13] == b':'
        && b[..4]
            .iter()
            .chain(&b[5..7])
            .chain(&b[8..10])
            .chain(&b[11..13])
            .chain(&b[14..16])
            .all(u8::is_ascii_digit);
    // Syncthing's "never" is year 1.
    (ok && !t.starts_with("0001")).then(|| format!("{} {}", &t[..10], &t[11..16]))
}

/// `myID` of `/rest/system/status`.
#[must_use]
pub fn parse_my_id(status: &Json) -> Option<String> {
    status.get("myID")?.as_str().map(str::to_owned)
}

/// The other devices: `/rest/config/devices` (the array) with
/// `/rest/system/connections`.
#[must_use]
pub fn parse_peers(devices: &Json, connections: &Json, my_id: &str) -> Vec<Peer> {
    let conns = connections.get("connections");
    devices
        .items()
        .iter()
        .filter_map(|d| {
            let id = d.get("deviceID")?.as_str()?;
            if id == my_id {
                return None;
            }
            let name = d.get("name").and_then(Json::as_str).unwrap_or("");
            Some(Peer {
                id: id.to_owned(),
                name: if name.is_empty() {
                    id.split('-').next().unwrap_or(id).to_owned()
                } else {
                    name.to_owned()
                },
                connected: conns
                    .and_then(|c| c.get(id))
                    .and_then(|c| c.get("connected"))
                    .and_then(Json::as_bool)
                    .unwrap_or(false),
            })
        })
        .collect()
}

/// `/rest/db/status` of the folder.
#[must_use]
pub fn parse_folder(id: &str, db: &Json) -> FolderView {
    let n = |k: &str| db.get(k).and_then(Json::as_u64).unwrap_or(0);
    FolderView {
        id: id.to_owned(),
        state: db
            .get("state")
            .and_then(Json::as_str)
            .unwrap_or("")
            .to_owned(),
        need_files: n("needFiles"),
        need_bytes: n("needBytes"),
        errors: n("pullErrors"),
        error: db
            .get("invalid")
            .and_then(Json::as_str)
            .unwrap_or("")
            .to_owned(),
    }
}

/// When a file last arrived in folder `id`: `/rest/stats/folder`.
#[must_use]
pub fn parse_last_sync(id: &str, stats: &Json) -> Option<String> {
    short_time(stats.get(id)?.get("lastFile")?.get("at")?.as_str()?)
}

/// `/rest/cluster/pending/devices`, at most [`MAX_PENDING`].
#[must_use]
pub fn parse_pending_devices(j: &Json) -> Vec<PendingDevice> {
    j.members()
        .iter()
        .take(MAX_PENDING)
        .map(|(id, v)| PendingDevice {
            id: id.clone(),
            name: v
                .get("name")
                .and_then(Json::as_str)
                .unwrap_or("")
                .to_owned(),
            address: v
                .get("address")
                .and_then(Json::as_str)
                .unwrap_or("")
                .to_owned(),
        })
        .collect()
}

/// `/rest/cluster/pending/folders`: one entry per folder and offering
/// device, at most [`MAX_PENDING`]. `known` names the devices already set up.
#[must_use]
pub fn parse_pending_folders(j: &Json, known: &[Peer]) -> Vec<PendingFolder> {
    let mut out = Vec::new();
    for (fid, v) in j.members() {
        let Some(by) = v.get("offeredBy") else {
            continue;
        };
        for (dev, o) in by.members() {
            if out.len() >= MAX_PENDING {
                return out;
            }
            out.push(PendingFolder {
                id: fid.clone(),
                label: o
                    .get("label")
                    .and_then(Json::as_str)
                    .unwrap_or("")
                    .to_owned(),
                device: dev.clone(),
                device_name: known
                    .iter()
                    .find(|p| p.id == *dev)
                    .map_or_else(String::new, |p| p.name.clone()),
            });
        }
    }
    out
}

/// The device ID in groups for reading: two lines of four groups.
#[must_use]
pub fn id_lines(id: &str) -> [String; 2] {
    let groups: Vec<&str> = id.split('-').collect();
    let half = groups.len().div_ceil(2);
    [groups[..half].join("-"), groups[half..].join("-")]
}

#[cfg(test)]
mod tests {
    use super::*;

    const ME: &str = "MFZWI3D-BONSGYC-YLTMRWG-C43ENR5-QXGZDMM-FZWI3DP-BONSGYY-LTMRWAD";
    const JOE: &str = "P56IOI7-MZJNU2Y-IQGDREY-DM2MGTI-MGL3BXN-PQ6W5BM-TBBZ4TJ-XZWICQ2";

    fn j(t: &str) -> Json {
        Json::parse(t).unwrap()
    }

    #[test]
    fn pending_lists_come_from_the_documented_shapes() {
        let d = parse_pending_devices(&j(&format!(
            r#"{{"{JOE}": {{"time": "2020-03-18T11:43:07Z", "name": "Friend Joe", "address": "192.168.1.2:22000"}}}}"#
        )));
        assert_eq!(d.len(), 1);
        assert_eq!((d[0].id.as_str(), d[0].name.as_str()), (JOE, "Friend Joe"));
        assert_eq!(d[0].address, "192.168.1.2:22000");

        let known = [Peer {
            id: JOE.into(),
            name: "Joe".into(),
            connected: true,
        }];
        let f = parse_pending_folders(
            &j(&format!(
                r#"{{"cpkn4-57ysy": {{"offeredBy": {{"{JOE}": {{"label": "Joe's folder"}}, "OTHER": {{"label": "Other"}}}}}}, "abcde-fghij": {{"offeredBy": {{"{JOE}": {{"label": "MyPics"}}}}}}}}"#
            )),
            &known,
        );
        assert_eq!(f.len(), 3);
        assert_eq!(
            (f[0].id.as_str(), f[0].label.as_str()),
            ("cpkn4-57ysy", "Joe's folder")
        );
        assert_eq!(f[0].device_name, "Joe");
        assert_eq!(f[1].device_name, "");
    }

    #[test]
    fn pending_lists_are_capped() {
        let many: String = (0..9)
            .map(|i| format!(r#""D{i}": {{"name": "n"}}"#))
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(
            parse_pending_devices(&j(&format!("{{{many}}}"))).len(),
            MAX_PENDING
        );
    }

    #[test]
    fn peers_skip_this_device_and_read_connected() {
        let devices = j(&format!(
            r#"[{{"deviceID": "{ME}", "name": "forma"}}, {{"deviceID": "{JOE}", "name": ""}}, {{"deviceID": "X-Y", "name": "Y"}}]"#
        ));
        let conns = j(&format!(
            r#"{{"connections": {{"{JOE}": {{"connected": true}}, "X-Y": {{"connected": false}}}}}}"#
        ));
        let p = parse_peers(&devices, &conns, ME);
        assert_eq!(p.len(), 2);
        assert_eq!((p[0].name.as_str(), p[0].connected), ("P56IOI7", true));
        assert_eq!((p[1].name.as_str(), p[1].connected), ("Y", false));
    }

    #[test]
    fn folder_and_last_sync() {
        let f = parse_folder(
            "notes",
            &j(
                r#"{"state": "syncing", "needFiles": 3, "needBytes": 122880, "pullErrors": 0, "invalid": ""}"#,
            ),
        );
        assert_eq!(
            (f.state.as_str(), f.need_files, f.need_bytes),
            ("syncing", 3, 122_880)
        );
        let stats = j(
            r#"{"notes": {"lastScan": "x", "lastFile": {"filename": "a.md", "at": "2026-10-28T14:05:33.3066971+01:00"}}}"#,
        );
        assert_eq!(
            parse_last_sync("notes", &stats).as_deref(),
            Some("2026-10-28 14:05")
        );
        let never = j(r#"{"notes": {"lastFile": {"at": "0001-01-01T00:00:00Z"}}}"#);
        assert_eq!(parse_last_sync("notes", &never), None);
        assert_eq!(parse_last_sync("other", &stats), None);
        assert_eq!(short_time("yesterday"), None);
        assert_eq!(short_time("2026-1x-28T14:05"), None);
    }

    #[test]
    fn remote_state_not_sharing_is_the_only_waiting_one() {
        let j = |t: &str| Json::parse(t).unwrap();
        assert!(not_sharing(&j(
            r#"{"completion":100,"remoteState":"notSharing"}"#
        )));
        for state in ["valid", "unknown", "paused"] {
            assert!(!not_sharing(&j(&format!(r#"{{"remoteState":"{state}"}}"#))));
        }
        assert!(!not_sharing(&j(r#"{"completion":100}"#)));
    }

    #[test]
    fn status_follows_the_state_of_things() {
        let mut v = View::default();
        assert_eq!(v.status(), Status::Off);
        v.run = Run::Starting;
        assert_eq!(v.status(), Status::Starting);
        v.run = Run::Running;
        assert_eq!(v.status(), Status::NoDevices);
        v.peers = vec![Peer {
            id: JOE.into(),
            name: "Joe".into(),
            connected: false,
        }];
        assert_eq!(v.status(), Status::NotConnected);
        v.peers[0].connected = true;
        // Connected, but nothing is shared: not "up to date".
        assert_eq!(v.status(), Status::NotShared);
        v.folder = Some(FolderView::default());
        assert_eq!(v.status(), Status::UpToDate);
        v.folder = None;
        v.peers[0].connected = false;
        assert_eq!(v.status(), Status::NotConnected);
        v.peers[0].connected = true;
        // A device that has not accepted the folder outranks "up to date" and
        // "scanning", and yields to errors and to syncing.
        v.folder = Some(FolderView::default());
        v.waiting = vec!["Joe".into()];
        assert_eq!(v.status(), Status::WaitingFor("Joe".into(), 0));
        v.waiting = vec!["Joe".into(), "Ann".into(), "Bo".into()];
        assert_eq!(v.status(), Status::WaitingFor("Joe".into(), 2));
        v.folder = Some(FolderView {
            state: "scanning".into(),
            ..FolderView::default()
        });
        assert_eq!(v.status(), Status::WaitingFor("Joe".into(), 2));
        v.folder = Some(FolderView {
            state: "syncing".into(),
            need_files: 2,
            need_bytes: 9,
            ..FolderView::default()
        });
        assert_eq!(v.status(), Status::Syncing(2, 9));
        v.folder = Some(FolderView {
            state: "error".into(),
            ..FolderView::default()
        });
        assert!(matches!(v.status(), Status::Error(_)));
        v.waiting.clear();
        v.folder = Some(FolderView {
            state: "syncing".into(),
            need_files: 2,
            need_bytes: 9,
            ..FolderView::default()
        });
        assert_eq!(v.status(), Status::Syncing(2, 9));
        v.folder = Some(FolderView {
            state: "scanning".into(),
            ..FolderView::default()
        });
        assert_eq!(v.status(), Status::Scanning);
        v.folder = Some(FolderView {
            state: "idle".into(),
            errors: 2,
            ..FolderView::default()
        });
        assert!(matches!(v.status(), Status::Error(_)));
        v.run = Run::Failed("exit 1".into());
        assert_eq!(v.status(), Status::Failed("exit 1".into()));
    }

    #[test]
    fn the_id_reads_in_two_lines() {
        let [a, b] = id_lines(JOE);
        assert_eq!(a, "P56IOI7-MZJNU2Y-IQGDREY-DM2MGTI");
        assert_eq!(b, "MGL3BXN-PQ6W5BM-TBBZ4TJ-XZWICQ2");
        assert_eq!(id_lines(""), [String::new(), String::new()]);
    }
}

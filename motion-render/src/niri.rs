//! Niri EventStream integration. Keep only layout/workspace state, never poll
//! windows per frame. Fullscreen is conservatively inferred from output-sized tiles.
use crate::wake::Wake;
use serde_json::Value;
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    sync::{mpsc, Arc},
    thread,
    time::Duration,
};
#[derive(Default)]
struct Model {
    windows: HashMap<u64, Value>,
    workspaces: HashMap<u64, Value>,
    overview: bool,
}
impl Model {
    fn event(&mut self, v: &Value) {
        if let Some(ws) = v
            .pointer("/WorkspacesChanged/workspaces")
            .and_then(Value::as_array)
        {
            self.workspaces.clear();
            for w in ws {
                if let Some(id) = w["id"].as_u64() {
                    self.workspaces.insert(id, w.clone());
                }
            }
        }
        if let Some(ws) = v
            .pointer("/WindowsChanged/windows")
            .and_then(Value::as_array)
        {
            self.windows.clear();
            for w in ws {
                if let Some(id) = w["id"].as_u64() {
                    self.windows.insert(id, w.clone());
                }
            }
        }
        if let Some(w) = v.get("WindowOpenedOrChanged").and_then(|x| x.get("window")) {
            if let Some(id) = w["id"].as_u64() {
                self.windows.insert(id, w.clone());
            }
        }
        if let Some(id) = v.pointer("/WindowClosed/id").and_then(Value::as_u64) {
            self.windows.remove(&id);
        }
        if let Some(e) = v.get("WorkspaceActivated") {
            if let Some(id) = e["id"].as_u64() {
                let out = self
                    .workspaces
                    .get(&id)
                    .and_then(|w| w["output"].as_str())
                    .map(str::to_owned);
                if let Some(out) = out {
                    for w in self.workspaces.values_mut() {
                        if w["output"].as_str() == Some(out.as_str()) {
                            w["is_active"] = Value::Bool(w["id"].as_u64() == Some(id));
                        }
                    }
                }
            }
        }
        if let Some(layouts) = v
            .pointer("/WindowLayoutsChanged/changes")
            .and_then(Value::as_array)
        {
            for item in layouts {
                if let (Some(id), Some(layout)) = (item[0].as_u64(), item.get(1)) {
                    if let Some(w) = self.windows.get_mut(&id) {
                        w["layout"] = layout.clone();
                    }
                }
            }
        }
        if let Some(open) = v
            .pointer("/OverviewOpenedOrClosed/is_open")
            .and_then(Value::as_bool)
        {
            self.overview = open;
        }
    }
    fn visible_tiles(&self) -> HashMap<String, Vec<(f64, f64)>> {
        let mut result: HashMap<String, Vec<(f64, f64)>> = HashMap::new();
        if self.overview {
            return result;
        }
        for w in self.windows.values() {
            let Some(ws) = w["workspace_id"]
                .as_u64()
                .and_then(|id| self.workspaces.get(&id))
            else {
                continue;
            };
            if !ws["is_active"].as_bool().unwrap_or(false) {
                continue;
            }
            let Some(out) = ws["output"].as_str() else {
                continue;
            };
            let Some(size) = w.pointer("/layout/tile_size").and_then(Value::as_array) else {
                continue;
            };
            if let (Some(width), Some(height)) = (
                size.first().and_then(Value::as_f64),
                size.get(1).and_then(Value::as_f64),
            ) {
                result
                    .entry(out.to_owned())
                    .or_default()
                    .push((width, height));
            }
        }
        // Stable ordering avoids wakeups for title/focus-only changes.
        for sizes in result.values_mut() {
            sizes.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
        }
        result
    }
}
pub type VisibleTiles = HashMap<String, Vec<(f64, f64)>>;

pub fn watch(wake: Arc<Wake>) -> mpsc::Receiver<VisibleTiles> {
    let (tx, rx) = mpsc::channel();
    let Some(path) = std::env::var_os("NIRI_SOCKET") else {
        log::warn!("NIRI_SOCKET missing: fullscreen integration disabled");
        return rx;
    };
    thread::spawn(move || loop {
        let run = || -> anyhow::Result<()> {
            let mut model = Model::default();
            let mut stream = UnixStream::connect(&path)?;
            stream.write_all(b"\"EventStream\"\n")?;
            let mut last = HashMap::new();
            for line in BufReader::new(stream).lines() {
                let v: Value = serde_json::from_str(&line?)?;
                model.event(&v);
                let now = model.visible_tiles();
                if now != last {
                    last = now.clone();
                    if tx.send(now).is_err() {
                        return Ok(());
                    }
                    wake.notify();
                }
            }
            Ok(())
        };
        if let Err(e) = run() {
            log::debug!("niri events: {e}");
        }
        if tx.send(HashMap::new()).is_err() {
            break;
        }
        wake.notify();
        thread::sleep(Duration::from_secs(5));
    });
    rx
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fullscreen_tracks_workspace_and_overview() {
        let mut m = Model::default();
        m.event(&serde_json::json!({"WorkspacesChanged":{"workspaces":[{"id":1,"output":"DP-1","is_active":true},{"id":2,"output":"DP-1","is_active":false}]}}));
        m.event(&serde_json::json!({"WindowsChanged":{"windows":[{"id":8,"workspace_id":1,"layout":{"tile_size":[1920,1080]}}]}}));
        assert_eq!(m.visible_tiles()["DP-1"], vec![(1920., 1080.)]);
        m.event(&serde_json::json!({"OverviewOpenedOrClosed":{"is_open":true}}));
        assert!(m.visible_tiles().is_empty());
        m.event(&serde_json::json!({"OverviewOpenedOrClosed":{"is_open":false}}));
        m.event(&serde_json::json!({"WorkspaceActivated":{"id":2}}));
        assert!(m.visible_tiles().is_empty());
    }
}

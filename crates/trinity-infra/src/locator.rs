use std::fs;

use trinity_app::error::AppError;
use trinity_app::ports::DeviceLocator;
use trinity_core::node::NodeId;

use crate::evdev_source::BY_ID_DIR;

/// Locates input nodes by their `/dev/input/by-id` name.
///
/// The prefix selects the device (e.g. `usb-Razer_Razer_Naga_Trinity`);
/// only `*-event-*` entries are kept (no `-mouse`, no `-hidraw`).
pub struct ByIdLocator {
    prefix: String,
    by_id_dir: String,
}

impl ByIdLocator {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            by_id_dir: BY_ID_DIR.to_owned(),
        }
    }

    /// Preset for the Razer Naga Trinity.
    pub fn trinity() -> Self {
        Self::new("usb-Razer_Razer_Naga_Trinity")
    }

    pub fn with_by_id_dir(mut self, dir: impl Into<String>) -> Self {
        self.by_id_dir = dir.into();
        self
    }

    pub fn prefix(&self) -> &str {
        &self.prefix
    }
}

impl Default for ByIdLocator {
    fn default() -> Self {
        Self::trinity()
    }
}

impl DeviceLocator for ByIdLocator {
    fn locate(&self) -> Result<Vec<NodeId>, AppError> {
        let entries = fs::read_dir(&self.by_id_dir)
            .map_err(|err| AppError::Port(format!("reading {}: {err}", self.by_id_dir)))?;
        let mut names = Vec::new();
        for entry in entries {
            let entry = entry
                .map_err(|err| AppError::Port(format!("reading {}: {err}", self.by_id_dir)))?;
            let Some(name) = entry.file_name().into_string().ok() else {
                continue;
            };
            if name.starts_with(&self.prefix) && name.contains("-event") {
                names.push(NodeId::new(name)?);
            }
        }
        names.sort_unstable();
        Ok(names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_nodes(dir: &std::path::Path, names: &[&str]) {
        std::fs::create_dir_all(dir).unwrap();
        for name in names {
            std::fs::write(dir.join(name), "").unwrap();
        }
    }

    #[test]
    fn keeps_only_event_nodes_with_prefix() {
        let temp = std::env::temp_dir().join(format!(
            "trinity-locator-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        write_nodes(
            &temp,
            &[
                "usb-Razer_Razer_Naga_Trinity_00000000001A-event-mouse",
                "usb-Razer_Razer_Naga_Trinity_00000000001A-if01-event-kbd",
                "usb-Razer_Razer_Naga_Trinity_00000000001A-if02-event-kbd",
                "usb-Razer_Razer_Naga_Trinity_00000000001A-mouse",
                "usb-Razer_Razer_Naga_Trinity_00000000001A-hidraw",
                "usb-Razer_Razer_Orbweaver-event-kbd",
            ],
        );
        let locator = ByIdLocator::trinity().with_by_id_dir(temp.to_str().unwrap());
        let nodes = locator.locate().unwrap();
        let names: Vec<&str> = nodes.iter().map(|n| n.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "usb-Razer_Razer_Naga_Trinity_00000000001A-event-mouse",
                "usb-Razer_Razer_Naga_Trinity_00000000001A-if01-event-kbd",
                "usb-Razer_Razer_Naga_Trinity_00000000001A-if02-event-kbd",
            ]
        );
        std::fs::remove_dir_all(&temp).unwrap();
    }

    #[test]
    fn live_trinity_is_located_when_present() {
        if !std::path::Path::new(BY_ID_DIR)
            .join("usb-Razer_Razer_Naga_Trinity_00000000001A-event-mouse")
            .exists()
        {
            return;
        }
        let nodes = ByIdLocator::trinity().locate().unwrap();
        assert_eq!(nodes.len(), 3);
    }

    #[test]
    fn empty_directory_yields_no_nodes() {
        let temp =
            std::env::temp_dir().join(format!("trinity-locator-empty-{}", std::process::id()));
        std::fs::create_dir_all(&temp).unwrap();
        let locator = ByIdLocator::trinity().with_by_id_dir(temp.to_str().unwrap());
        assert!(locator.locate().unwrap().is_empty());
        std::fs::remove_dir_all(&temp).unwrap();
    }
}

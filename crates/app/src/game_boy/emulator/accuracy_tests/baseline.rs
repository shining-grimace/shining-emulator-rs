use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Status {
    Pass,
    Fail,
    Timeout,
    Stopped,
    InvalidOpcode,
}

impl Status {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Timeout => "timeout",
            Self::Stopped => "stopped",
            Self::InvalidOpcode => "invalid-opcode",
        }
    }
}

pub(super) fn path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/game_boy/emulator/accuracy_tests/baseline.tsv")
}

pub(super) fn parse(text: &str) -> Result<BTreeMap<String, Status>, String> {
    let mut baseline = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 3 || !matches!(fields[0], "DMG" | "SGB" | "CGB") {
            return Err(format!("invalid baseline row {}: {line:?}", index + 1));
        }
        let status = match fields[1] {
            "pass" => Status::Pass,
            "fail" => Status::Fail,
            "timeout" => Status::Timeout,
            "stopped" => Status::Stopped,
            "invalid-opcode" => Status::InvalidOpcode,
            other => return Err(format!("unknown baseline status: {other}")),
        };
        if !fields[2].starts_with("acceptance/")
            || std::path::Path::new(fields[2])
                .extension()
                .is_none_or(|ext| ext != "gb")
        {
            return Err(format!("invalid baseline ROM: {}", fields[2]));
        }
        let key = format!("{}\t{}", fields[0], fields[2]);
        if baseline.insert(key.clone(), status).is_some() {
            return Err(format!("duplicate baseline case: {key}"));
        }
    }
    Ok(baseline)
}

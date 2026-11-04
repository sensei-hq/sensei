//! A retention setting as a prune may use it — shared by every pruner (D83
//! batch A, audit rows 148-151 and 155).

/// A stored value parsed, no value the default — and a failed read an error,
/// so the pass deletes nothing rather than deleting under a default nobody
/// chose.
#[allow(dead_code)] // used by the pruners' prune_once in D83 batch A phase B
pub fn setting(
    read: Result<Option<String>, String>,
    parse: impl FnOnce(Option<String>) -> i32,
) -> Result<i32, String> {
    let _ = (read, parse);
    todo!("D83 batch A phase B")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn days(v: Option<String>) -> i32 {
        v.map_or(90, |s| s.trim().parse().unwrap())
    }

    #[test]
    #[ignore = "pending: D83 batch A"]
    fn a_setting_that_cannot_be_read_is_an_error() {
        assert_eq!(setting(Err("pool timed out".into()), days), Err("pool timed out".into()));
    }

    #[test]
    #[ignore = "pending: D83 batch A"]
    fn a_stored_value_is_parsed_and_an_unset_one_is_the_default() {
        assert_eq!(setting(Ok(Some(" 45 ".into())), days), Ok(45));
        assert_eq!(setting(Ok(None), days), Ok(90));
    }
}

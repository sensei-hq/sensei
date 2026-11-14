//! A retention setting as a prune may use it — shared by every pruner: activity,
//! git history and logs (D83 batch A, audit rows 148-151, 155 and 182).

/// A stored value parsed, no value the default — and a failed read an error,
/// so the pass deletes nothing rather than deleting under a default nobody
/// chose.
pub fn setting(
    read: Result<Option<String>, String>,
    parse: impl FnOnce(Option<String>) -> i32,
) -> Result<i32, String> {
    read.map(parse)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn days(v: Option<String>) -> i32 {
        v.map_or(90, |s| s.trim().parse().unwrap())
    }

    #[test]
    fn a_setting_that_cannot_be_read_is_an_error() {
        assert_eq!(setting(Err("pool timed out".into()), days), Err("pool timed out".into()));
    }

    #[test]
    fn a_stored_value_is_parsed_and_an_unset_one_is_the_default() {
        assert_eq!(setting(Ok(Some(" 45 ".into())), days), Ok(45));
        assert_eq!(setting(Ok(None), days), Ok(90));
    }
}

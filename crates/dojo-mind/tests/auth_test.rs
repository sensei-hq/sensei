use dojo_mind::auth::{role_satisfies, Role};

#[test]
fn role_floor_ordering() {
    assert!(role_satisfies(Role::Admin, Role::Publisher));
    assert!(role_satisfies(Role::Publisher, Role::Member));
    assert!(role_satisfies(Role::Member, Role::Member));
    assert!(!role_satisfies(Role::Member, Role::Publisher));
    assert!(!role_satisfies(Role::Publisher, Role::Admin));
}

#[test]
fn role_parses_from_db_text() {
    assert_eq!(Role::parse("admin"), Some(Role::Admin));
    assert_eq!(Role::parse("publisher"), Some(Role::Publisher));
    assert_eq!(Role::parse("member"), Some(Role::Member));
    assert_eq!(Role::parse("nonsense"), None);
}

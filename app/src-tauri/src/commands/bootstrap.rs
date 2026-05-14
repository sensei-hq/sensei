//! Bootstrap commands — placeholder pending Task G2.
//!
//! `check_and_fix_bootstrap` and its helpers (emit_gate, emit_phase_complete,
//! dispatch, write_bootstrap_session, collect_system_info) referenced legacy
//! symbols from sensei-bootstrap that were removed in the health rewrite
//! (BootstrapReport, prereq::*, check_and_fix). They are deleted here so that
//! src-tauri compiles cleanly.
//!
//! Task G2 will fill this file with the two new health commands:
//!   * health_check        — fast path, returns HealthPayload
//!   * health_check_and_resolve — streaming check + fix pipeline

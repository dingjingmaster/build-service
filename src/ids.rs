use uuid::Uuid;

pub fn build_id() -> String {
    format!("build_{}", short_uuid())
}

pub fn run_id() -> String {
    format!("run_{}", short_uuid())
}

pub fn terminal_session_id() -> String {
    format!("term_{}", short_uuid())
}

pub fn upgrade_id() -> String {
    format!("upgrade_{}", short_uuid())
}

fn short_uuid() -> String {
    Uuid::new_v4().as_simple().to_string()
}

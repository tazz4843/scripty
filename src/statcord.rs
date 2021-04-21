use std::sync::atomic::AtomicU64;

pub struct StatcordData {
    servers: u64,
    users: u128,
    commands: AtomicU64,
    total_audio_transcript_minutes: u128,
    active_voice_connections: u64,
}

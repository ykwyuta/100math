fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-changed=assets/game.rc");
        println!("cargo:rerun-if-changed=assets/game.ico");
        embed_windows_icon();
    }
}

// Build scripts run on the host, so the helper must also be available when
// Windows is the build target and the host is Linux.
fn embed_windows_icon() {
    embed_resource::compile("assets/game.rc", embed_resource::NONE)
        .manifest_required()
        .expect("could not embed the Windows game icon");
}

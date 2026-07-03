fn main() {
    slint_build::compile("ui/app.slint").expect("Slint build failed");
    let _ = embed_resource::compile("assets/icon.rc", embed_resource::NONE);
}

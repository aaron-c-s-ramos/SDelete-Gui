fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");

    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "SDelete Gui");
        res.set(
            "FileDescription",
            "Windows Explorer context-menu manager for Microsoft Sysinternals SDelete",
        );
        res.compile()
            .expect("Failed to compile Windows icon resource.");
    }
}

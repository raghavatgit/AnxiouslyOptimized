fn main() {
    #[cfg(target_os = "windows")]
    {
        let is_release = std::env::var("PROFILE").as_deref() == Ok("release");
        println!("cargo:rerun-if-env-changed=PROFILE");
        println!("cargo:rerun-if-changed=app.manifest");
        println!("cargo:rerun-if-changed=assets/icon.ico");

        let mut res = winresource::WindowsResource::new();
        for candidate in [
            r"D:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64",
            r"C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64",
        ] {
            if std::path::Path::new(candidate).join("rc.exe").exists() {
                res.set_toolkit_path(candidate);
                break;
            }
        }
        res.set_icon("assets/icon.ico");
        if is_release {
            res.set_manifest_file("app.manifest");
        }
        res.set("ProductName", "Optimizer by Raghav");
        res.set("FileDescription", "Optimizer by Raghav - Windows System Identifier & Optimizer");
        res.set("CompanyName", "Raghav Goyal");
        res.set("LegalCopyright", "Copyright (C) 2026 Raghav Goyal");
        res.set("OriginalFilename", "OptimizerByRaghav.exe");
        res.set("ProductVersion", "2.2.0.0");
        res.set("FileVersion", "2.2.0.0");

        if let Err(e) = res.compile() {
            panic!("Failed to compile Windows resources: {e}");
        }
    }
}

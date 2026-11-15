fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "JSFS - Journal Scraper For Students");
        res.set(
            "FileDescription",
            "JSFS Academic Journal Finder & APC Validator",
        );
        res.set("CompanyName", "JSFS Open Source");
        res.set("OriginalFilename", "jsfs.exe");
        res.set("InternalName", "jsfs");
        res.set(
            "LegalCopyright",
            "Copyright (c) 2026 JSFS Contributors - MIT License",
        );
        res.set("FileVersion", "4.8.1.0");
        res.set("ProductVersion", "4.8.1.0");
        res.set_manifest(
            r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0" xmlns:asmv3="urn:schemas-microsoft-com:asm.v3">
  <assemblyIdentity version="4.8.1.0" processorArchitecture="*" name="JSFS.OpenSource.Desktop" type="win32" />
  <description>JSFS Academic Journal Finder and APC Validator</description>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false" />
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <!-- Windows 10 and Windows 11 -->
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}" />
    </application>
  </compatibility>
  <dependency>
    <dependentAssembly>
      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0" processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*" />
    </dependentAssembly>
  </dependency>
  <asmv3:application>
    <asmv3:windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2,PerMonitor</dpiAwareness>
    </asmv3:windowsSettings>
  </asmv3:application>
</assembly>"#,
        );
        if let Err(e) = res.compile() {
            println!("cargo:warning=winres resource compilation skipped: {e}");
        }
    }
}

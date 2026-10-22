//! Вшивает иконку и версию в Windows-exe (панель задач, Проводник).
fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "Kvass Shop");
        res.set("FileDescription", "Kvass Shop");
        res.set("OriginalFilename", "kvass-shop.exe");
        res.compile().expect("winres: не удалось вшить иконку");
    }
}

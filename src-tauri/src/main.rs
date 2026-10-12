// Evita una consola extra en Windows en las compilaciones de release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    proxy_builder_lib::run()
}

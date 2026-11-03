// Ningún comando para la vista web: el plugin solo lo llama Rust
// (`src/android.rs` de la app), así que no hay permisos que declarar.
const COMMANDS: &[&str] = &[];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .build();
}

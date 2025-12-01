fn main() {
    println!("Environment variables (JetsonScope):");
    for (k, v) in std::env::vars() {
        if k.starts_with("JETSONSCOPE_") || k.starts_with("TEGRA_") {
            println!("{}={}", k, v);
        }
    }
}

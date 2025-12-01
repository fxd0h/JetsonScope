use jetsonscope::parser::{SizeUnit, TegraStats};

fn parse(line: &str) -> TegraStats {
    TegraStats::parse(line).expect("failed to parse sample")
}

#[test]
fn parses_orin_timestamp_and_ram() {
    let line = "01-03-2023 16:10:22 RAM 2257/30536MB (lfb 5392x4MB) SWAP 0/15268MB (cached 0MB) CPU [10%@729,20%@729,30%@729,40%@729,50%@729,0%@729,60%@729,70%@729,80%@729,90%@729,100%@729,0%@729] EMC_FREQ 0% GR3D_FREQ 75% CV0@-256C CPU@41.375C Tboard@29C SOC2@39C Tdiode@30.75C SOC0@38.906C CV1@-256C GPU@-256C tj@41.468C SOC1@38.843C CV2@-256C";
    let stats = parse(line);
    assert_eq!(stats.timestamp.as_deref(), Some("01-03-2023 16:10:22"));
    let ram = stats.ram.as_ref().unwrap();
    assert_eq!(ram.total_bytes, SizeUnit::MB.to_bytes(30_536));
    assert_eq!(ram.used_bytes, SizeUnit::MB.to_bytes(2_257));
    assert_eq!(stats.cpus.len(), 12);
    assert_eq!(stats.cpus[0].freq_mhz, Some(729));
    assert_eq!(stats.gpu_usage(), Some(75));
}

#[test]
fn parses_power_and_engines_xavier() {
    let line = "RAM 4181/7771MB (lfb 8x4MB) SWAP 0/3885MB (cached 0MB) CPU [10%@1190,0%@1190,1%@1190,0%@1190,5%@1190,1%@1190] EMC_FREQ 15%@1600 GR3D_FREQ 0% PLL@42.906C Tdiode@43.25C Tboard@36C GPU@41.75C BCPU@42.5C MCPU@47.5C thermal@42.425C VDD_SYS_GPU 47mW/0mW VDD_SYS_SOC 813mW/207mW VDD_4V0_WIFI 495mW/0mW VDD_IN 3539mW/1422mW VDD_SYS_CPU 125mW/104mW";
    let stats = parse(line);
    let ram = stats.ram.as_ref().unwrap();
    assert_eq!(ram.total_bytes, SizeUnit::MB.to_bytes(7_771));
    assert!(stats.power.contains_key("VDD_IN"));
    assert!(stats.engines.get("GR3D").is_some());
}

#[test]
fn parses_fanless_orin_nano_sample() {
    let line = "RAM 624/1999MB (lfb 7x4MB) SWAP 0/999MB (cached 0MB) CPU [2%@1190,1%@1190,0%@1190,0%@1190,1%@1190,0%@1190] EMC_FREQ 0%@1600 GR3D_FREQ 0%@318 NVDEC 0 NVENC 0 VIC_FREQ 0%@1152 APE 0 PLL@38.0C Tboard@31C Tdiode@34.5C AUX@32.5C thermal@38.12C VDD_SYS_GPU 42mW/0mW VDD_SYS_SOC 528mW/245mW VDD_4V0_WIFI 0mW/0mW VDD_IN 2235mW/1684mW VDD_SYS_CPU 119mW/106mW";
    let stats = parse(line);
    assert_eq!(stats.cpus.len(), 6);
    assert!(stats.engines.contains_key("NVDEC"));
    assert!(stats.engines.contains_key("NVENC"));
    assert!(stats.power.contains_key("VDD_IN"));
}

#[test]
fn parses_negative_temps_and_missing_values() {
    let line = "RAM 4181/7771MB (lfb 8x4MB) SWAP 0/3885MB (cached 0MB) CPU [10%@1190,0%@1190,1%@1190,0%@1190,5%@1190,1%@1190] EMC_FREQ 15%@1600 GR3D_FREQ 0% CV0@-256C CPU@41.375C GPU@-256C";
    let stats = parse(line);
    assert!(stats.temps.contains_key("CPU"));
    assert_eq!(stats.temps.get("GPU"), Some(&-256.0));
}

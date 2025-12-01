use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum SizeUnit {
    KB,
    #[default]
    MB,
}


impl SizeUnit {
    fn from_suffix(raw: &str) -> Option<Self> {
        match raw {
            "k" | "K" => Some(SizeUnit::KB),
            "M" | "m" => Some(SizeUnit::MB),
            _ => None,
        }
    }

    pub fn to_bytes(self, value: u64) -> u64 {
        match self {
            SizeUnit::KB => value.saturating_mul(1024),
            SizeUnit::MB => value.saturating_mul(1024 * 1024),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryStat {
    pub used_bytes: u64,
    pub total_bytes: u64,
    pub unit: SizeUnit,
    pub largest_free_block: Option<LargestFreeBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum LargestFreeBlock {
    Blocks { count: u64, size_bytes: u64 },
    Size { size_bytes: u64 },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SwapStat {
    pub used_bytes: u64,
    pub total_bytes: u64,
    pub cached_bytes: Option<u64>,
    pub unit: SizeUnit,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IramStat {
    pub used_bytes: u64,
    pub total_bytes: u64,
    pub lfb_bytes: Option<u64>,
    pub unit: SizeUnit,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CpuCore {
    pub load_percent: Option<u32>,
    pub freq_mhz: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EngineStat {
    pub usage_percent: Option<u32>,
    pub freq_mhz: Option<u32>,
    pub raw_value: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PowerRail {
    pub current_mw: u32,
    pub average_mw: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MtsStat {
    pub fg_percent: u32,
    pub bg_percent: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TegraStats {
    pub timestamp: Option<String>,
    pub ram: Option<MemoryStat>,
    pub swap: Option<SwapStat>,
    pub iram: Option<IramStat>,
    pub mts: Option<MtsStat>,
    pub cpus: Vec<CpuCore>,
    pub engines: HashMap<String, EngineStat>,
    pub temps: HashMap<String, f32>,
    pub power: HashMap<String, PowerRail>,
    #[allow(dead_code)]
    pub raw: String,
}

impl TegraStats {
    pub fn parse(line: &str) -> Result<Self> {
        let raw = line.trim().to_string();
        let mut stats = TegraStats {
            raw: raw.clone(),
            ..Default::default()
        };

        let mut payload = raw.clone();

        if let Some(mat) = DATE_RE.find(&payload) {
            stats.timestamp = Some(mat.as_str().trim().to_string());
            payload.replace_range(mat.range(), "");
            payload = payload.trim_start().to_string();
        }

        stats.ram = parse_ram(&payload);
        stats.swap = parse_swap(&payload);
        stats.iram = parse_iram(&payload);
        stats.mts = parse_mts(&payload);
        stats.cpus = parse_cpus(&payload);
        stats.engines = parse_engines(&payload);
        stats.temps = parse_temps(&payload);
        stats.power = parse_power(&payload);

        Ok(stats)
    }

    #[allow(dead_code)]
    pub fn ram_ratio(&self) -> f64 {
        self.ram
            .as_ref()
            .map(|ram| {
                if ram.total_bytes == 0 {
                    0.0
                } else {
                    ram.used_bytes as f64 / ram.total_bytes as f64
                }
            })
            .unwrap_or_default()
    }

    #[allow(dead_code)]
    pub fn swap_ratio(&self) -> f64 {
        self.swap
            .as_ref()
            .map(|swap| {
                if swap.total_bytes == 0 {
                    0.0
                } else {
                    swap.used_bytes as f64 / swap.total_bytes as f64
                }
            })
            .unwrap_or_default()
    }

    pub fn gpu_usage(&self) -> Option<u32> {
        self.engines
            .get("GR3D")
            .and_then(|e| e.usage_percent.or(e.raw_value))
    }
}

static DATE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\d{2}-\d{2}-\d{4} \d{2}:\d{2}:\d{2}").unwrap());
static SWAP_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"SWAP (\d+)/(\d+)(\w)B ?\(cached (\d+)(\w)B\)").unwrap());
static IRAM_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"IRAM (\d+)/(\d+)(\w)B ?\(lfb (\d+)(\w)B\)").unwrap());
static RAM_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"RAM (\d+)/(\d+)(\w)B ?\(lfb (\d+)x(\d+)(\w)B\)").unwrap());
static MTS_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"MTS fg (\d+)% bg (\d+)%").unwrap());
static VALS_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b([A-Z0-9_]+) ([0-9%@]+(?:@\[\d+\]|@\d+)?)\b").unwrap());
static ENGINE_OFF_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b([A-Z0-9_]+) off\b").unwrap());
static BRACKET_FREQ_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"([A-Z0-9_]+) ([0-9]+)%@\[(\d+)\]").unwrap());
static UTIL_ONLY_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"([A-Z0-9_]+_UTIL) ([0-9]+)%").unwrap());
static VAL_FREQ_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\d+)%@(\d+)").unwrap());
static CPU_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"CPU \[(.*?)\]").unwrap());
static WATT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b(\w+) ([0-9.]+)(\w?)W?/([0-9.]+)(\w?)W?\b").unwrap());
static TEMP_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b(\w+)@(-?[0-9.]+)C\b").unwrap());

fn parse_size_unit(raw: &str) -> SizeUnit {
    SizeUnit::from_suffix(raw).unwrap_or(SizeUnit::MB)
}

fn parse_ram(text: &str) -> Option<MemoryStat> {
    RAM_RE.captures(text).map(|caps| {
        let unit = parse_size_unit(&caps[3]);
        let used = caps[1].parse::<u64>().unwrap_or_default();
        let total = caps[2].parse::<u64>().unwrap_or_default();
        let lfb_count = caps[4].parse::<u64>().unwrap_or_default();
        let lfb_size = caps[5].parse::<u64>().unwrap_or_default();
        let lfb_unit = parse_size_unit(&caps[6]);
        let largest_free_block = Some(LargestFreeBlock::Blocks {
            count: lfb_count,
            size_bytes: lfb_unit.to_bytes(lfb_size),
        });

        MemoryStat {
            used_bytes: unit.to_bytes(used),
            total_bytes: unit.to_bytes(total),
            unit,
            largest_free_block,
        }
    })
}

fn parse_swap(text: &str) -> Option<SwapStat> {
    SWAP_RE.captures(text).map(|caps| {
        let unit = parse_size_unit(&caps[3]);
        let cached_unit = parse_size_unit(&caps[5]);
        let used = caps[1].parse::<u64>().unwrap_or_default();
        let total = caps[2].parse::<u64>().unwrap_or_default();
        let cached = caps[4].parse::<u64>().unwrap_or_default();

        SwapStat {
            used_bytes: unit.to_bytes(used),
            total_bytes: unit.to_bytes(total),
            cached_bytes: Some(cached_unit.to_bytes(cached)),
            unit,
        }
    })
}

fn parse_iram(text: &str) -> Option<IramStat> {
    IRAM_RE.captures(text).map(|caps| {
        let unit = parse_size_unit(&caps[3]);
        let lfb_unit = parse_size_unit(&caps[5]);
        let used = caps[1].parse::<u64>().unwrap_or_default();
        let total = caps[2].parse::<u64>().unwrap_or_default();
        let lfb = caps[4].parse::<u64>().unwrap_or_default();

        IramStat {
            used_bytes: unit.to_bytes(used),
            total_bytes: unit.to_bytes(total),
            lfb_bytes: Some(lfb_unit.to_bytes(lfb)),
            unit,
        }
    })
}

fn parse_mts(text: &str) -> Option<MtsStat> {
    MTS_RE.captures(text).map(|caps| MtsStat {
        fg_percent: caps[1].parse::<u32>().unwrap_or_default(),
        bg_percent: caps[2].parse::<u32>().unwrap_or_default(),
    })
}

fn parse_val_freq(val: &str) -> EngineStat {
    if let Some((usage_part, freq_part)) = val.split_once('@') {
        let usage = usage_part.trim_end_matches('%').parse::<u32>().ok();
        let freq_clean = freq_part.trim_matches(['[', ']']);
        let freq = freq_clean.parse::<u32>().ok();
        return EngineStat {
            usage_percent: usage,
            freq_mhz: freq,
            raw_value: None,
        };
    }
    if let Some(caps) = VAL_FREQ_RE.captures(val) {
        let usage = caps[1].parse::<u32>().ok();
        let freq = caps[2].parse::<u32>().ok();
        EngineStat {
            usage_percent: usage,
            freq_mhz: freq,
            raw_value: None,
        }
    } else if let Some(stripped) = val.strip_suffix('%') {
        EngineStat {
            usage_percent: stripped.parse::<u32>().ok(),
            freq_mhz: None,
            raw_value: None,
        }
    } else {
        EngineStat {
            usage_percent: None,
            freq_mhz: val.parse::<u32>().ok(),
            raw_value: val.parse::<u32>().ok(),
        }
    }
}

fn parse_engines(text: &str) -> HashMap<String, EngineStat> {
    let mut engines = HashMap::new();
    for caps in BRACKET_FREQ_RE.captures_iter(text) {
        let mut name = caps[1].to_string();
        if let Some(stripped) = name.strip_suffix("_FREQ") {
            name = stripped.to_string();
        }
        if matches!(name.as_str(), "RAM" | "SWAP" | "IRAM" | "CPU" | "MTS") {
            continue;
        }
        let usage = caps[2].parse::<u32>().ok();
        let freq = caps[3].parse::<u32>().ok();
        engines.entry(name).or_insert_with(|| EngineStat {
            usage_percent: usage,
            freq_mhz: freq,
            raw_value: None,
        });
    }
    for caps in VALS_RE.captures_iter(text) {
        let mut name = caps[1].to_string();
        if let Some(stripped) = name.strip_suffix("_FREQ") {
            name = stripped.to_string();
        }
        // Skip fields handled elsewhere
        if matches!(name.as_str(), "RAM" | "SWAP" | "IRAM" | "CPU" | "MTS") {
            continue;
        }
        let val = caps[2].to_string();
        // Skip UTIL tokens here; handled in UTIL_ONLY_RE to avoid double-insert
        if name.ends_with("_UTIL") {
            continue;
        }
        engines.entry(name).or_insert_with(|| parse_val_freq(&val));
    }
    // UTIL-only tokens (e.g., NVCSI_UTIL 3%)
    for caps in UTIL_ONLY_RE.captures_iter(text) {
        let mut name = caps[1].to_string();
        if matches!(name.as_str(), "RAM" | "SWAP" | "IRAM" | "CPU" | "MTS") {
            continue;
        }
        let usage = caps[2].parse::<u32>().ok();
        engines.entry(name.clone()).or_insert_with(|| EngineStat {
            usage_percent: usage,
            freq_mhz: None,
            raw_value: None,
        });
        if let Some(stripped) = name.strip_suffix("_UTIL") {
            let base = stripped.to_string();
            engines.entry(base).or_insert_with(|| EngineStat {
                usage_percent: usage,
                freq_mhz: None,
                raw_value: None,
            });
        }
    }
    for caps in ENGINE_OFF_RE.captures_iter(text) {
        let name = caps[1].to_string();
        if matches!(name.as_str(), "RAM" | "SWAP" | "IRAM" | "CPU" | "MTS") {
            continue;
        }
        engines.entry(name).or_insert_with(|| EngineStat {
            usage_percent: Some(0),
            freq_mhz: None,
            raw_value: None,
        });
    }
    engines
}

fn parse_cpus(text: &str) -> Vec<CpuCore> {
    if let Some(caps) = CPU_RE.captures(text) {
        let content = caps[1].split(',');
        let mut cpus = Vec::new();
        for cpu_str in content {
            let cpu_str = cpu_str.trim();
            if cpu_str == "off" {
                cpus.push(CpuCore::default());
                continue;
            }
            let mut core = CpuCore::default();
            if let Some(caps) = VAL_FREQ_RE.captures(cpu_str) {
                core.load_percent = caps[1].parse::<u32>().ok();
                core.freq_mhz = caps[2].parse::<u32>().ok();
            } else if let Some(stripped) = cpu_str.strip_suffix('%') {
                core.load_percent = stripped.parse::<u32>().ok();
            }
            cpus.push(core);
        }
        cpus
    } else {
        Vec::new()
    }
}

fn parse_temps(text: &str) -> HashMap<String, f32> {
    TEMP_RE
        .captures_iter(text)
        .filter_map(|caps| {
            let name = caps[1].to_string();
            let val = caps[2].parse::<f32>().ok()?;
            Some((name, val))
        })
        .collect()
}

fn normalize_power(unit: &str, value: f64) -> u32 {
    match unit {
        "m" | "M" => value as u32,
        "k" | "K" => (value * 1_000.0) as u32,
        "" => value as u32,
        _ => (value * 1_000.0) as u32,
    }
}

fn parse_power(text: &str) -> HashMap<String, PowerRail> {
    let mut rails = HashMap::new();
    for caps in WATT_RE.captures_iter(text) {
        let name = caps[1].to_string();
        let cur_val = caps[2].parse::<f64>().unwrap_or_default();
        let cur_unit = caps[3].to_string();
        let avg_val = caps[4].parse::<f64>().unwrap_or_default();
        let avg_unit = caps[5].to_string();

        rails.insert(
            name,
            PowerRail {
                current_mw: normalize_power(&cur_unit, cur_val),
                average_mw: normalize_power(&avg_unit, avg_val),
            },
        );
    }
    rails
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_orin_sample_with_timestamp() {
        let line = "01-03-2023 16:10:22 RAM 2257/30536MB (lfb 5392x4MB) SWAP 0/15268MB (cached 0MB) CPU [10%@729,20%@729,30%@729,40%@729,50%@729,0%@729,60%@729,70%@729,80%@729,90%@729,100%@729,0%@729] EMC_FREQ 0% GR3D_FREQ 75% CV0@-256C CPU@41.375C Tboard@29C SOC2@39C Tdiode@30.75C SOC0@38.906C CV1@-256C GPU@-256C tj@41.468C SOC1@38.843C CV2@-256C";
        let stats = TegraStats::parse(line).unwrap();

        assert_eq!(stats.timestamp.as_deref(), Some("01-03-2023 16:10:22"));
        let ram = stats.ram.as_ref().unwrap();
        assert_eq!(ram.total_bytes, SizeUnit::MB.to_bytes(30_536));
        assert_eq!(ram.used_bytes, SizeUnit::MB.to_bytes(2_257));
        assert!(stats.swap.is_some());
        assert_eq!(stats.cpus.len(), 12);
        assert_eq!(stats.cpus[0].load_percent, Some(10));
        assert_eq!(stats.cpus[0].freq_mhz, Some(729));
        assert_eq!(stats.cpus[10].load_percent, Some(100));
        assert_eq!(stats.gpu_usage(), Some(75));
        assert!(stats.temps.contains_key("CPU"));
        assert!(stats.temps.contains_key("tj"));
    }

    #[test]
    fn parses_power_and_engines() {
        let line = "RAM 4722/7844MB (lfb 1x512kB) CPU [12%@2035,34%@2034,56%@2034,78%@2035,90%@2035,99%@2035] SWAP 149/1024MB (cached 7MB) EMC_FREQ 2%@1866 GR3D_FREQ 59%@1300 APE 150 MTS fg 3% bg 9% BCPU@-45C MCPU@-45C GPU@-51C PLL@45C AO@47.5C Tboard@37C Tdiode@46.75C PMIC@100C thermal@46.4C VDD_IN 14025/14416 VDD_CPU 2209/2538 VDD_GPU 6854/6903 VDD_SOC 1371/1370 VDD_WIFI 19/19 NVENC 716 NVDEC 716 VDD_DDR 2702/2702";
        let stats = TegraStats::parse(line).unwrap();

        assert_eq!(stats.cpus.len(), 6);
        assert_eq!(stats.cpus[1].load_percent, Some(34));
        assert_eq!(stats.cpus[1].freq_mhz, Some(2034));
        assert_eq!(
            stats.engines.get("EMC").and_then(|e| e.freq_mhz),
            Some(1866)
        );
        assert_eq!(
            stats.engines.get("GR3D").and_then(|e| e.usage_percent),
            Some(59)
        );
        assert_eq!(
            stats.engines.get("NVDEC").and_then(|e| e.raw_value),
            Some(716)
        );
        assert_eq!(stats.mts.as_ref().map(|m| m.fg_percent), Some(3));
        let power = stats.power.get("VDD_IN").unwrap();
        assert_eq!(power.current_mw, 14025);
        assert_eq!(power.average_mw, 14416);
    }

    #[test]
    fn parses_verbose_off_and_bracket_freqs() {
        let line = "11-30-2025 13:26:01 RAM 2461/7620MB (lfb 3x2MB) SWAP 1243/3810MB (cached 5MB) CPU [19%@729,14%@729,22%@729,8%@729,15%@729,17%@729] EMC_FREQ 4%@2133 GR3D_FREQ 0%@[305] NVDEC off NVJPG off NVJPG1 off VIC off OFA off APE 200 cpu@46.531C soc2@47.312C soc0@46.593C gpu@48.218C tj@48.843C soc1@48.843C VDD_IN 5704mW/5704mW VDD_CPU_GPU_CV 831mW/831mW VDD_SOC 1624mW/1624mW";
        let stats = TegraStats::parse(line).unwrap();

        assert_eq!(stats.engines.get("EMC").and_then(|e| e.usage_percent), Some(4));
        assert_eq!(stats.engines.get("EMC").and_then(|e| e.freq_mhz), Some(2133));
        assert_eq!(stats.engines.get("GR3D").and_then(|e| e.freq_mhz), Some(305));
        assert_eq!(stats.engines.get("GR3D").and_then(|e| e.usage_percent), Some(0));
        assert_eq!(stats.engines.get("NVDEC").and_then(|e| e.usage_percent), Some(0));
        assert_eq!(stats.engines.get("NVJPG1").and_then(|e| e.usage_percent), Some(0));
        assert_eq!(stats.engines.get("VIC").and_then(|e| e.usage_percent), Some(0));
        assert_eq!(stats.engines.get("APE").and_then(|e| e.raw_value), Some(200));
    }

    #[test]
    fn parses_extended_engines_from_reference() {
        let line = "RAM 1024/4096MB (lfb 1x1MB) SWAP 0/1024MB (cached 0MB) CPU [10%@1200,20%@1200] EMC_FREQ 25%@1600 MC_FREQ 800 AXI_FREQ 600 GR3D_FREQ 50%@900 NVENC 30%@700 NVDEC 15%@650 NVJPG off NVJPG1 5%@300 VIC 12%@400 OFA 7%@350 ISP 9%@500 NVCSI 3%@250 PCIE 1%@125 NVLINK 2%@400 ISP_UTIL 4% NVCSI_UTIL 6% VDD_IN 5000/5200";
        let stats = TegraStats::parse(line).unwrap();

        assert_eq!(stats.engines.get("EMC").and_then(|e| e.usage_percent), Some(25));
        assert_eq!(stats.engines.get("EMC").and_then(|e| e.freq_mhz), Some(1600));
        assert_eq!(stats.engines.get("MC").and_then(|e| e.freq_mhz), Some(800));
        assert_eq!(stats.engines.get("AXI").and_then(|e| e.freq_mhz), Some(600));
        assert_eq!(stats.engines.get("GR3D").and_then(|e| e.usage_percent), Some(50));
        assert_eq!(stats.engines.get("GR3D").and_then(|e| e.freq_mhz), Some(900));
        assert_eq!(stats.engines.get("NVENC").and_then(|e| e.usage_percent), Some(30));
        assert_eq!(stats.engines.get("NVENC").and_then(|e| e.freq_mhz), Some(700));
        assert_eq!(stats.engines.get("NVDEC").and_then(|e| e.usage_percent), Some(15));
        assert_eq!(stats.engines.get("NVDEC").and_then(|e| e.freq_mhz), Some(650));
        assert_eq!(stats.engines.get("NVJPG").and_then(|e| e.usage_percent), Some(0));
        assert_eq!(stats.engines.get("NVJPG1").and_then(|e| e.usage_percent), Some(5));
        assert_eq!(stats.engines.get("VIC").and_then(|e| e.usage_percent), Some(12));
        assert_eq!(stats.engines.get("OFA").and_then(|e| e.usage_percent), Some(7));
        assert_eq!(stats.engines.get("ISP").and_then(|e| e.usage_percent), Some(9));
        assert_eq!(stats.engines.get("NVCSI").and_then(|e| e.usage_percent), Some(3));
        assert_eq!(stats.engines.get("PCIE").and_then(|e| e.usage_percent), Some(1));
        assert_eq!(stats.engines.get("NVLINK").and_then(|e| e.usage_percent), Some(2));
        assert_eq!(stats.engines.get("NVCSI_UTIL").and_then(|e| e.usage_percent), Some(6));
        assert_eq!(stats.engines.get("ISP_UTIL").and_then(|e| e.usage_percent), Some(4));
    }
}

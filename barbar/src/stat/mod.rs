use std::path::Path;
use systemstat::{CPULoad, Platform};


pub struct BarStatAggregator {
    sys: systemstat::System,
    cpu_stat: Option<systemstat::DelayedMeasurement<CPULoad>>,
}

impl Default for BarStatAggregator {
    fn default() -> Self {
        Self::new()
    }
}

impl BarStatAggregator {
    pub fn new() -> Self {
        let sys = systemstat::System::new();
        let cpu_stat = sys.cpu_load_aggregate().ok();
        BarStatAggregator {
            sys,
            cpu_stat,
        }
    }

    pub fn get_bar_stat(&mut self) -> anyhow::Result<BarStat> {
        let root_path = Path::new("/");
        let cpu = self.get_cpu_stat()?;
        let disk = get_disk_usage(root_path)?;
        let mem = self.get_mem_stat()?;
        Ok(BarStat {
            cpu,
            disk,
            mem
        })
    }

    fn get_cpu_stat(&mut self) -> anyhow::Result<CpuStat> {
        let temp = self.sys.cpu_temp()?;
        if let Some(cpu_stat) = self.cpu_stat.take() {
            let cpu_stat = cpu_stat.done()?;
            self.cpu_stat = self.sys.cpu_load_aggregate().ok();
            return Ok(CpuStat {
                temp,
                user: cpu_stat.user,
                nice: cpu_stat.nice,
                system: cpu_stat.system,
                interrupt: cpu_stat.interrupt,
                idle: cpu_stat.idle
            })
        }

        Ok(CpuStat {
            temp,
            user: 0.0,
            nice: 0.0,
            system: 0.0,
            interrupt: 0.0,
            idle: 0.0
        })
    }

    fn get_mem_stat(&self) -> anyhow::Result<MemoryUsageStat> {
        let mem = self.sys.memory()?;
        Ok(MemoryUsageStat { total: mem.total.as_u64(), free: mem.free.as_u64() })
    }
}

pub struct BarStat {
    pub cpu: CpuStat,
    pub disk: DiskUsageStat,
    pub mem: MemoryUsageStat
}

pub enum Unit {
    GB,
    MB,
    KB,
    B
}

impl Unit {
    pub fn as_bytes(&self) -> u64 {
        match self {
            Unit::GB => 1024 * 1024 * 1024,
            Unit::MB => 1024 * 1024,
            Unit::KB => 1024,
            Unit::B => 1
        }
    }
}

impl BarStat {
    pub fn disk_used(&self, unit: Unit) -> f32 {
        (self.disk.total - self.disk.free) as f32 / unit.as_bytes() as f32
    }

    pub fn disk_total(&self, unit: Unit) -> f32 {
        self.disk.total as f32 / unit.as_bytes() as f32
    }

    pub fn disk_used_pct(&self) -> f32 {
        if self.disk_total(Unit::B) == 0.0 {
            return 0.0;
        }

        (self.disk_used(Unit::B) / self.disk_total(Unit::B)) * 100.0
    }

    pub fn cpu_usage_pct(&self) -> f32 {
        // 100.0 - self.cpu.idle * 100.0
        self.cpu.system * 100.0 
            + self.cpu.user * 100.0 
            + self.cpu.nice * 100.0 
            + self.cpu.interrupt * 100.0
    }

    pub fn mem_usage_pct(&self) -> f32 {
        let used = self.mem.total - self.mem.free;
        ((used as f32) / (self.mem.total as f32)) * 100.0
    }

    pub fn cpu_temp(&self) -> f32 {
        self.cpu.temp
    }
}

pub struct CpuStat {
    pub temp: f32,
    pub user: f32,
    pub nice: f32,
    pub system: f32,
    pub interrupt: f32,
    pub idle: f32,
}

pub struct DiskUsageStat {
    pub total: u64,
    pub avail: u64,
    pub free: u64
}

pub struct MemoryUsageStat {
    pub total: u64,
    pub free: u64
}

fn get_disk_usage<P: AsRef<Path>>(path: P) -> anyhow::Result<DiskUsageStat> {
    let stats = rustix::fs::statvfs(path.as_ref())?;
    let total = stats.f_blocks * stats.f_frsize;
    let avail = stats.f_bavail * stats.f_frsize;
    let free = stats.f_bfree * stats.f_frsize;
    Ok(DiskUsageStat { total, avail, free })
}


use clap::Parser;
use cli_table::{CellStruct, format::Justify, print_stdout, Table, WithTitle};
use histogram::Histogram;
use lazy_static::lazy_static;
use regex::Regex;
use std::fs::File;
use std::io::{BufReader, BufRead};
use std::path::PathBuf;

lazy_static! {
    static ref RE_ALLOC: Regex = Regex::new(r"allocation request:\s(?P<alloc>\d{6,}) bytes,.*source:\sconcurrent\shumongous\sallocation\]$").unwrap();
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(required = true, validator = is_file)]
    files: Vec<PathBuf>,
}

#[derive(Table)]
struct G1RegionBucket {
    #[table(title = "Region Size", justify = "Justify::Right")]
    region_size: String,
    #[table(title = "Max Allocation Size (50%)")]
    max_size: u32,
    #[table(title = "Number of Allocations")]
    num_allocations: u32,
}

fn is_file(path: &str) -> Result<(), String> {
    if std::path::Path::new(path).is_file() { return Ok(()); }
    Err(String::from(format!("{} is not a valid file", path)))
}

// Manual string parsing implementation
// Faster than Python's regex
fn parse_humongous_object_allocation(line: &str) -> Option<u64> {
    match line.split_once("allocation request: ") {
        Some(alloc_split) => {
            if alloc_split.1.ends_with("source: concurrent humongous allocation]") {
                match alloc_split.1.split_once(" bytes,") {
                    Some(alloc) => {
                        return Some(alloc.0.parse::<u64>().unwrap());
                    }
                    None => return None
                }
            } else { return None };
        }
        None => return None
    }
}

// Original implementation that ended up being much too slow on large files (Python outperforms it)
// Captures use a 3 pass system (find the match, find boundaries, determine captures)
// No additional string parsing needed
fn parse_humongous_object_allocation_with_regex_captures(line: &str) -> Option<u64> {
    match RE_ALLOC.captures(line) {
        Some(caps) => {
            let alloc = caps.name("alloc").unwrap().as_str().parse::<u64>().unwrap();
            Some(alloc)
        }
        None => None,
    }
}

// Second implementation to speed up regex parsing
// Still too slow (Python outperforms)
// Find uses a 2 pass system (find the match, find the boundaries)
// Implementation must apply additional string parsing on top
fn parse_humongous_object_allocation_with_regex_find(line: &str) -> Option<u64> {
    match RE_ALLOC.find(line) {
        Some(alloc) => {
            let alloc = alloc.as_str().split_once(" bytes").unwrap().0;
            Some(alloc[20..].parse::<u64>().unwrap())
        }
        None => None
    }
}

fn extract_region_size(file: &PathBuf) -> Result<u32, String> {
    let gc_log = File::open(file).expect(format!("ERROR: Unable to open {:?}", file).as_str());
    match BufReader::new(gc_log).lines().nth(3) {
        Some(line) => {
            let third_line = line.unwrap();
            if third_line.contains("PrintAdaptiveSizePolicy") {
                let region_size: Vec<(&str, &str)> = third_line.split(" -XX:").filter(|x| x.contains("G1HeapRegionSize")).map(|x| x.split_once("=").unwrap()).collect();
                return Ok(region_size[0].1.parse::<u32>().unwrap() / 1024 / 1024);
            } else {
                return Err("ERROR: Humongous allocation sizes are not being printed in the provided gc log. Please add -XX:PrintAdaptiveSizePolicy in order to print out humongous allocation sizes".to_string());
            }
        },
        None => return Err(format!("ERROR: File {:?} did not contain 3+ lines, does not appear to be a valid gc log", file)),
    }
    
}

fn gather_humongous_object_allocations(file: &PathBuf, allocs_histogram: &mut Histogram, region_size_array: &mut [G1RegionBucket; 6]) {
    let file_region_size = extract_region_size(&file);
    if file_region_size.is_err() {
        eprintln!("{:?}", file_region_size.unwrap_err());
    } else {
        println!("Region Size: {}MB - {:?}", file_region_size.unwrap(), file);

        let gc_log_buf = BufReader::new(File::open(file).expect("Unable to open file"));

        let allocations: Vec<_> = gc_log_buf
            .lines()
            .filter_map(|line| line.ok())
            .map(|x| parse_humongous_object_allocation(&x))
            .filter_map(|x| x)
            .collect();
        for item in allocations {
            allocs_histogram.increment(item);
            match item {
                //G1 region size of 2MB
                524289..=1048576 => region_size_array[0].num_allocations += 1,
                // G1 region size of 4MB
                1048577..=2097152 => region_size_array[1].num_allocations += 1,
                // G1 region size of 8MB
                2097153..=4194304 => region_size_array[2].num_allocations += 1,
                // G1 region size of 16MB
                4194305..=8388608 => region_size_array[3].num_allocations += 1,
                // G1 region size of 32MB
                8388609..=16777216 => region_size_array[4].num_allocations += 1,
                // Everything that is bigger than 50% of 32MB
                16777217..=u64::MAX => region_size_array[5].num_allocations += 1,
                // Catch any 0 byte allocations or anything for a 1MB region because that should never happen
                _ => eprintln!("WARN: Unexpected byte allocation <= 524289 occurred in the log"),
            }
        }
    }
}

fn main() {
    let args = Cli::parse();

    let mut allocs_histogram = Histogram::new();
    let mut region_size_array = [
        G1RegionBucket { region_size: "2MB".to_string(), max_size: 1048576, num_allocations: 0},
        G1RegionBucket { region_size: "4MB".to_string(), max_size: 2097152, num_allocations: 0},
        G1RegionBucket { region_size: "8MB".to_string(), max_size: 4194304, num_allocations: 0},
        G1RegionBucket { region_size: "16MB".to_string(), max_size: 8388608, num_allocations: 0},
        G1RegionBucket { region_size: "32MB".to_string(),  max_size: 16777216, num_allocations: 0},
        G1RegionBucket { region_size: "Overflow".to_string(), max_size: u32::MAX, num_allocations: 0}
    ];

    for file in args.files {
        gather_humongous_object_allocations(&file, &mut allocs_histogram, &mut region_size_array);
    }
    if region_size_array.iter().map(|x| x.num_allocations).sum::<u32>() > 0 {
        print_stdout(region_size_array.with_title());
        println!("\nAllocation Size Percentiles:\n\tmin: {}\n\tp50: {}\n\tp75: {}\n\tp90: {}\n\tp99: {}\n\tmax: {}",
            allocs_histogram.minimum().unwrap(),
            allocs_histogram.percentile(50.0).unwrap(),
            allocs_histogram.percentile(75.0).unwrap(),
            allocs_histogram.percentile(90.0).unwrap(),
            allocs_histogram.percentile(99.0).unwrap(),
            allocs_histogram.maximum().unwrap(),
        );
    } else {
        println!("\nNo humongous allocations were identified in the provided data set.")
    }

}
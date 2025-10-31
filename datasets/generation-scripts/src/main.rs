use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use rand::prelude::*;
use rand_distr::Normal;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "datagen")]
#[command(about = "Generate and validate synthetic FASTA/FASTQ datasets for ASBB", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate synthetic FASTA or FASTQ file
    Generate {
        /// Output file path
        #[arg(short, long)]
        output: PathBuf,

        /// File format (fasta or fastq)
        #[arg(short, long, value_parser = ["fasta", "fastq"])]
        format: String,

        /// Number of sequences to generate
        #[arg(short, long)]
        num_sequences: usize,

        /// Mean sequence length (bp)
        #[arg(short, long, default_value = "150")]
        length_mean: usize,

        /// Standard deviation of sequence length (bp)
        #[arg(short, long, default_value = "10")]
        length_std: f64,

        /// Quality score distribution (uniform_high, degrading, realistic)
        #[arg(short, long, default_value = "degrading", value_parser = ["uniform_high", "degrading", "realistic"])]
        quality_dist: String,

        /// Random seed for reproducibility
        #[arg(short, long, default_value = "42")]
        seed: u64,

        /// Run QC validation after generation
        #[arg(long, default_value = "true")]
        validate: bool,
    },

    /// Validate existing FASTA or FASTQ file
    Validate {
        /// Input file to validate
        #[arg(short, long)]
        input: PathBuf,

        /// Expected file format (fasta or fastq)
        #[arg(short, long, value_parser = ["fasta", "fastq"])]
        format: String,

        /// Output validation report JSON
        #[arg(short, long)]
        report: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate {
            output,
            format,
            num_sequences,
            length_mean,
            length_std,
            quality_dist,
            seed,
            validate,
        } => {
            println!("🧬 Generating {} {} sequences...", num_sequences, format.to_uppercase());
            println!("   Output: {}", output.display());
            println!("   Mean length: {}bp ± {}bp", length_mean, length_std);
            println!("   Quality distribution: {}", quality_dist);
            println!("   Seed: {}", seed);

            let result = if format == "fasta" {
                generate_fasta(&output, num_sequences, length_mean, length_std, seed)
            } else {
                generate_fastq(
                    &output,
                    num_sequences,
                    length_mean,
                    length_std,
                    &quality_dist,
                    seed,
                )
            };

            result.context("Failed to generate sequences")?;

            println!("✅ Generation complete!");

            if validate {
                println!("\n🔍 Running QC validation...");
                validate_file(&output, &format, None)?;
            }
        }

        Commands::Validate {
            input,
            format,
            report,
        } => {
            println!("🔍 Validating {} file: {}", format.to_uppercase(), input.display());
            validate_file(&input, &format, report.as_ref())?;
        }
    }

    Ok(())
}

/// Generate synthetic FASTA file
fn generate_fasta(
    output: &PathBuf,
    num_sequences: usize,
    length_mean: usize,
    length_std: f64,
    seed: u64,
) -> Result<()> {
    let mut rng = StdRng::seed_from_u64(seed);
    let length_dist = Normal::new(length_mean as f64, length_std)
        .context("Invalid length distribution parameters")?;

    let file = File::create(output).context("Failed to create output file")?;
    let mut writer = BufWriter::new(file);

    let pb = ProgressBar::new(num_sequences as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    for i in 0..num_sequences {
        // Sample sequence length (clamp to reasonable range)
        let length = length_dist.sample(&mut rng).round() as usize;
        let length = length.clamp(50, length_mean * 2);

        // Generate header
        writeln!(writer, ">seq_{}", i)?;

        // Generate sequence (60 characters per line, standard FASTA wrapping)
        let sequence = generate_random_sequence(&mut rng, length);
        for chunk in sequence.as_bytes().chunks(60) {
            writer.write_all(chunk)?;
            writeln!(writer)?;
        }

        pb.inc(1);
    }

    pb.finish_with_message("Done!");
    writer.flush()?;

    Ok(())
}

/// Generate synthetic FASTQ file
fn generate_fastq(
    output: &PathBuf,
    num_sequences: usize,
    length_mean: usize,
    length_std: f64,
    quality_dist: &str,
    seed: u64,
) -> Result<()> {
    let mut rng = StdRng::seed_from_u64(seed);
    let length_dist = Normal::new(length_mean as f64, length_std)
        .context("Invalid length distribution parameters")?;

    let file = File::create(output).context("Failed to create output file")?;
    let mut writer = BufWriter::new(file);

    let pb = ProgressBar::new(num_sequences as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    for i in 0..num_sequences {
        // Sample sequence length
        let length = length_dist.sample(&mut rng).round() as usize;
        let length = length.clamp(50, length_mean * 2);

        // Generate sequence
        let sequence = generate_random_sequence(&mut rng, length);

        // Generate quality scores (CRITICAL: must match sequence length exactly!)
        let quality = generate_quality_scores(&mut rng, length, quality_dist);

        // FASTQ format: exactly 4 lines per record, no wrapping
        writeln!(writer, "@seq_{}", i)?; // Line 1: Header (starts with @)
        writeln!(writer, "{}", sequence)?; // Line 2: Sequence (no wrapping!)
        writeln!(writer, "+")?; // Line 3: Plus line (can be empty or repeat header)
        writeln!(writer, "{}", quality)?; // Line 4: Quality (must match sequence length!)

        pb.inc(1);
    }

    pb.finish_with_message("Done!");
    writer.flush()?;

    Ok(())
}

/// Generate random DNA sequence
fn generate_random_sequence(rng: &mut StdRng, length: usize) -> String {
    const BASES: &[u8] = b"ACGT";
    (0..length)
        .map(|_| BASES[rng.gen_range(0..4)] as char)
        .collect()
}

/// Generate quality scores with specified distribution
fn generate_quality_scores(rng: &mut StdRng, length: usize, dist: &str) -> String {
    match dist {
        "uniform_high" => {
            // Q40 (Phred+33 = 'I')
            "I".repeat(length)
        }
        "degrading" => {
            // Q40 → Q20 over read length (realistic Illumina pattern)
            (0..length)
                .map(|i| {
                    let q = 40.0 - (20.0 * i as f64 / length as f64);
                    let phred = (q as u8).clamp(0, 40);
                    (phred + 33) as char // Phred+33 encoding
                })
                .collect()
        }
        "realistic" => {
            // Simulate realistic Illumina distribution
            // High quality (Q35-40) with occasional drops
            (0..length)
                .map(|_| {
                    let base_q = rng.gen_range(35..=40);
                    // 5% chance of quality drop
                    let q = if rng.gen::<f64>() < 0.05 {
                        rng.gen_range(20..30)
                    } else {
                        base_q
                    };
                    (q + 33) as u8 as char
                })
                .collect()
        }
        _ => unreachable!("Invalid quality distribution"),
    }
}

/// Validate FASTA or FASTQ file
fn validate_file(
    input: &PathBuf,
    format: &str,
    report_path: Option<&PathBuf>,
) -> Result<()> {
    let result = if format == "fasta" {
        validate_fasta(input)
    } else {
        validate_fastq(input)
    };

    match result {
        Ok(report) => {
            println!("\n✅ Validation PASSED!");
            println!("{}", report);

            if let Some(path) = report_path {
                let json = serde_json::to_string_pretty(&report)?;
                std::fs::write(path, json)?;
                println!("\n📄 Report saved to: {}", path.display());
            }

            Ok(())
        }
        Err(e) => {
            println!("\n❌ Validation FAILED!");
            println!("{:#}", e);
            bail!("Validation failed");
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct ValidationReport {
    file_format: String,
    num_records: usize,
    min_length: usize,
    max_length: usize,
    mean_length: f64,
    issues: Vec<String>,
}

impl std::fmt::Display for ValidationReport {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "📊 Validation Report:")?;
        writeln!(f, "   Format: {}", self.file_format)?;
        writeln!(f, "   Records: {}", self.num_records)?;
        writeln!(f, "   Length range: {}-{} bp", self.min_length, self.max_length)?;
        writeln!(f, "   Mean length: {:.1} bp", self.mean_length)?;
        if self.issues.is_empty() {
            writeln!(f, "   Issues: None")?;
        } else {
            writeln!(f, "   Issues: {}", self.issues.len())?;
            for issue in &self.issues {
                writeln!(f, "     - {}", issue)?;
            }
        }
        Ok(())
    }
}

/// Validate FASTA file
fn validate_fasta(input: &PathBuf) -> Result<ValidationReport> {
    let file = File::open(input).context("Failed to open input file")?;
    let reader = BufReader::new(file);

    let mut num_records = 0;
    let mut lengths = Vec::new();
    let mut issues = Vec::new();
    let mut current_seq = String::new();
    let mut in_sequence = false;
    let mut line_num = 0;

    for line_result in reader.lines() {
        line_num += 1;
        let line = line_result.context(format!("Failed to read line {}", line_num))?;

        if line.starts_with('>') {
            // Header line
            if in_sequence && !current_seq.is_empty() {
                // Save previous sequence
                lengths.push(current_seq.len());
                current_seq.clear();
            }
            num_records += 1;
            in_sequence = true;

            if line.trim() == ">" {
                issues.push(format!("Line {}: Empty header", line_num));
            }
        } else if in_sequence {
            // Sequence line
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue; // Skip empty lines
            }

            // Validate sequence characters
            for (i, ch) in trimmed.chars().enumerate() {
                if !matches!(ch, 'A' | 'C' | 'G' | 'T' | 'N' | 'a' | 'c' | 'g' | 't' | 'n') {
                    issues.push(format!(
                        "Line {}, pos {}: Invalid character '{}' in sequence",
                        line_num,
                        i + 1,
                        ch
                    ));
                }
            }

            current_seq.push_str(trimmed);
        }
    }

    // Save last sequence
    if in_sequence && !current_seq.is_empty() {
        lengths.push(current_seq.len());
    }

    if num_records == 0 {
        bail!("No sequences found in file");
    }

    if lengths.is_empty() {
        bail!("No sequence data found (only headers)");
    }

    let min_length = *lengths.iter().min().unwrap();
    let max_length = *lengths.iter().max().unwrap();
    let mean_length = lengths.iter().sum::<usize>() as f64 / lengths.len() as f64;

    if lengths.len() != num_records {
        issues.push(format!(
            "Sequence count mismatch: {} headers, {} sequences",
            num_records,
            lengths.len()
        ));
    }

    Ok(ValidationReport {
        file_format: "FASTA".to_string(),
        num_records,
        min_length,
        max_length,
        mean_length,
        issues,
    })
}

/// Validate FASTQ file
fn validate_fastq(input: &PathBuf) -> Result<ValidationReport> {
    let file = File::open(input).context("Failed to open input file")?;
    let reader = BufReader::new(file);

    let mut num_records = 0;
    let mut lengths = Vec::new();
    let mut issues = Vec::new();
    let mut lines = reader.lines();
    let mut record_num = 0;

    loop {
        record_num += 1;
        let line_base = (record_num - 1) * 4 + 1;

        // Line 1: Header (must start with @)
        let header = match lines.next() {
            Some(Ok(line)) => line,
            Some(Err(e)) => bail!("Failed to read line {}: {}", line_base, e),
            None => break, // End of file
        };

        if !header.starts_with('@') {
            issues.push(format!(
                "Line {}: Header must start with '@', got '{}'",
                line_base,
                header.chars().next().unwrap_or(' ')
            ));
        }

        // Line 2: Sequence
        let sequence = match lines.next() {
            Some(Ok(line)) => line,
            Some(Err(e)) => bail!("Failed to read line {}: {}", line_base + 1, e),
            None => bail!("Incomplete FASTQ record at line {}", line_base),
        };

        let seq_len = sequence.len();
        lengths.push(seq_len);

        // Validate sequence characters
        for (i, ch) in sequence.chars().enumerate() {
            if !matches!(ch, 'A' | 'C' | 'G' | 'T' | 'N' | 'a' | 'c' | 'g' | 't' | 'n') {
                issues.push(format!(
                    "Line {}, pos {}: Invalid character '{}' in sequence",
                    line_base + 1,
                    i + 1,
                    ch
                ));
            }
        }

        // Line 3: Plus line (must start with +)
        let plus = match lines.next() {
            Some(Ok(line)) => line,
            Some(Err(e)) => bail!("Failed to read line {}: {}", line_base + 2, e),
            None => bail!("Incomplete FASTQ record at line {}", line_base),
        };

        if !plus.starts_with('+') {
            issues.push(format!(
                "Line {}: Plus line must start with '+', got '{}'",
                line_base + 2,
                plus.chars().next().unwrap_or(' ')
            ));
        }

        // Line 4: Quality scores (CRITICAL: must match sequence length!)
        let quality = match lines.next() {
            Some(Ok(line)) => line,
            Some(Err(e)) => bail!("Failed to read line {}: {}", line_base + 3, e),
            None => bail!("Incomplete FASTQ record at line {}", line_base),
        };

        let qual_len = quality.len();

        // CRITICAL CHECK: quality length must equal sequence length
        if qual_len != seq_len {
            issues.push(format!(
                "Record {}: Quality length ({}) does not match sequence length ({})",
                record_num, qual_len, seq_len
            ));
        }

        // Validate quality score characters (Phred+33: 33-126)
        for (i, ch) in quality.chars().enumerate() {
            let ascii = ch as u8;
            if ascii < 33 || ascii > 126 {
                issues.push(format!(
                    "Line {}, pos {}: Invalid quality character '{}' (ASCII {})",
                    line_base + 3,
                    i + 1,
                    ch,
                    ascii
                ));
            }
        }

        num_records += 1;
    }

    if num_records == 0 {
        bail!("No sequences found in file");
    }

    let min_length = *lengths.iter().min().unwrap();
    let max_length = *lengths.iter().max().unwrap();
    let mean_length = lengths.iter().sum::<usize>() as f64 / lengths.len() as f64;

    Ok(ValidationReport {
        file_format: "FASTQ".to_string(),
        num_records,
        min_length,
        max_length,
        mean_length,
        issues,
    })
}

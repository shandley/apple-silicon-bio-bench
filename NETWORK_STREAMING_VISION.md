# Network Streaming Vision: Eliminating the Storage Barrier

**Data Access Pillar Implementation**

---

## The Problem: Storage Accessibility Crisis

### Current State

**Scenario**: Researcher wants to analyze SRA dataset SRR1234567 (5TB compressed)

**Traditional workflow**:
1. Download 5TB to local storage ($150-1500 for drives)
2. Wait days/weeks (slow connections, unreliable networks)
3. Decompress to ~15TB (3× expansion)
4. Load into RAM for analysis (requires 12-24 TB RAM)

**Total cost**: $50K-100K (storage + RAM + server)

**Who this excludes**:
- LMIC researchers (limited bandwidth, expensive storage)
- Students (no budget for infrastructure)
- Small labs (shared laptops, not servers)
- Field researchers (portable devices only)

### The biofast Solution

**New workflow**:
```rust
biofast::stream::from_sra("SRR1234567")?
    .filter_quality(30)
    .gc_content()
    .compute()?
```

**Requirements**:
- 24GB RAM laptop ($1,400)
- 50GB storage (smart cache)
- Internet connection (any speed)

**Cost savings**: $48K-98K (98-99% reduction)

---

## Architecture Overview

### Three Streaming Layers

1. **Memory Streaming** (Week 1-2, v0.1.0)
   - Stream from local files
   - Constant memory (<100 MB)
   - Block-based processing (10K chunks)

2. **Network Streaming** (Week 3-4, v0.2.0)
   - Stream from HTTP/HTTPS URLs
   - Smart LRU caching
   - Background prefetching
   - Resume on network failure

3. **SRA Integration** (Week 3-4, v0.2.0)
   - Stream from NCBI SRA accessions
   - SRA Toolkit integration
   - Metadata handling

### Design Principles

1. **Transparent**: User API identical for local/HTTP/SRA
2. **Efficient**: Smart caching + prefetching (minimize downloads)
3. **Resilient**: Resume on failure (unreliable networks)
4. **Configurable**: User controls cache size budget

---

## HTTP/HTTPS Streaming (Week 3)

### Core Abstraction: StreamSource

```rust
pub enum StreamSource {
    LocalFile(PathBuf),
    Http { url: Url, cache: HttpCache },
    Sra { accession: String, cache: SraCache },
}

impl StreamSource {
    pub fn open(input: &str) -> Result<Self> {
        match input {
            path if Path::new(path).exists() => {
                Ok(StreamSource::LocalFile(path.into()))
            }
            url if url.starts_with("http://") || url.starts_with("https://") => {
                Ok(StreamSource::Http {
                    url: Url::parse(url)?,
                    cache: HttpCache::default()?,
                })
            }
            acc if acc.starts_with("SRR") || acc.starts_with("ERR") || acc.starts_with("DRR") => {
                Ok(StreamSource::Sra {
                    accession: acc.to_string(),
                    cache: SraCache::default()?,
                })
            }
            _ => Err(Error::InvalidInput(input.to_string())),
        }
    }

    pub fn into_reader(self) -> Result<Box<dyn Read + Send>> {
        match self {
            StreamSource::LocalFile(path) => {
                Ok(Box::new(File::open(path)?))
            }
            StreamSource::Http { url, cache } => {
                Ok(Box::new(HttpReader::new(url, cache)?))
            }
            StreamSource::Sra { accession, cache } => {
                Ok(Box::new(SraReader::new(accession, cache)?))
            }
        }
    }
}
```

### HTTP Reader with Range Requests

**Concept**: Download file in chunks, not all at once

```rust
pub struct HttpReader {
    url: Url,
    cache: HttpCache,

    // Current state
    current_chunk: Option<Vec<u8>>,
    chunk_offset: usize,
    file_offset: u64,
    file_size: Option<u64>,

    // Prefetching
    prefetch_handle: Option<JoinHandle<Result<Vec<u8>>>>,
    prefetch_distance: usize,  // How many chunks ahead to prefetch
}

impl HttpReader {
    pub fn new(url: Url, cache: HttpCache) -> Result<Self> {
        // HEAD request to get file size
        let client = reqwest::blocking::Client::new();
        let response = client.head(url.as_str()).send()?;
        let file_size = response.headers()
            .get(CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok());

        Ok(Self {
            url,
            cache,
            current_chunk: None,
            chunk_offset: 0,
            file_offset: 0,
            file_size,
            prefetch_handle: None,
            prefetch_distance: 2,  // Prefetch 2 chunks ahead
        })
    }

    fn fetch_chunk(&mut self, offset: u64, size: usize) -> Result<Vec<u8>> {
        // Check cache first
        if let Some(cached) = self.cache.get(& self.url, offset, size)? {
            return Ok(cached);
        }

        // Not in cache, download via range request
        let client = reqwest::blocking::Client::new();
        let end = offset + size as u64 - 1;

        let response = client
            .get(self.url.as_str())
            .header(RANGE, format!("bytes={}-{}", offset, end))
            .send()?;

        if !response.status().is_success() {
            return Err(Error::HttpError(response.status()));
        }

        let data = response.bytes()?.to_vec();

        // Store in cache
        self.cache.put(&self.url, offset, &data)?;

        Ok(data)
    }

    fn start_prefetch(&mut self) {
        // Prefetch next chunk in background
        let next_offset = self.file_offset + self.current_chunk.as_ref().map_or(0, |c| c.len() as u64);
        let chunk_size = self.cache.chunk_size();

        let url = self.url.clone();
        let cache = self.cache.clone();

        self.prefetch_handle = Some(std::thread::spawn(move || {
            // This runs in background while current chunk is being processed
            Self::fetch_chunk_static(&url, &cache, next_offset, chunk_size)
        }));
    }
}

impl Read for HttpReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // If no current chunk, fetch one
        if self.current_chunk.is_none() {
            // Check if prefetch completed
            if let Some(handle) = self.prefetch_handle.take() {
                self.current_chunk = Some(handle.join().unwrap()?);
            } else {
                // No prefetch, fetch synchronously
                let chunk = self.fetch_chunk(self.file_offset, self.cache.chunk_size())?;
                self.current_chunk = Some(chunk);
            }

            self.chunk_offset = 0;

            // Start prefetching next chunk
            self.start_prefetch();
        }

        // Read from current chunk
        let chunk = self.current_chunk.as_ref().unwrap();
        let available = chunk.len() - self.chunk_offset;
        let to_read = buf.len().min(available);

        buf[..to_read].copy_from_slice(&chunk[self.chunk_offset..self.chunk_offset + to_read]);

        self.chunk_offset += to_read;
        self.file_offset += to_read as u64;

        // If chunk exhausted, clear it
        if self.chunk_offset >= chunk.len() {
            self.current_chunk = None;
        }

        Ok(to_read)
    }
}
```

### Smart LRU Cache

**Purpose**: Minimize bandwidth while enabling large-scale analysis

```rust
pub struct HttpCache {
    cache_dir: PathBuf,           // ~/.biofast/cache/http/
    max_size_bytes: u64,          // User-configurable (default: 50 GB)
    chunk_size: usize,            // 1 MB chunks

    // LRU tracking
    access_log: Arc<Mutex<VecDeque<CacheEntry>>>,
    current_size: Arc<AtomicU64>,
}

struct CacheEntry {
    url_hash: String,
    offset: u64,
    size: usize,
    last_access: SystemTime,
    file_path: PathBuf,
}

impl HttpCache {
    pub fn default() -> Result<Self> {
        let cache_dir = dirs::home_dir()
            .ok_or(Error::NoCacheDir)?
            .join(".biofast")
            .join("cache")
            .join("http");

        std::fs::create_dir_all(&cache_dir)?;

        Ok(Self {
            cache_dir,
            max_size_bytes: 50 * 1024 * 1024 * 1024,  // 50 GB default
            chunk_size: 1024 * 1024,  // 1 MB
            access_log: Arc::new(Mutex::new(VecDeque::new())),
            current_size: Arc::new(AtomicU64::new(0)),
        })
    }

    pub fn with_max_size(mut self, bytes: u64) -> Self {
        self.max_size_bytes = bytes;
        self
    }

    pub fn get(&self, url: &Url, offset: u64, size: usize) -> Result<Option<Vec<u8>>> {
        let url_hash = Self::hash_url(url);
        let chunk_id = offset / self.chunk_size as u64;
        let cache_file = self.cache_dir
            .join(&url_hash)
            .join(format!("chunk_{:016x}.bin", chunk_id));

        if !cache_file.exists() {
            return Ok(None);
        }

        // Update access time (for LRU)
        self.touch(&url_hash, chunk_id)?;

        // Read from cache
        let data = std::fs::read(&cache_file)?;
        Ok(Some(data))
    }

    pub fn put(&self, url: &Url, offset: u64, data: &[u8]) -> Result<()> {
        let url_hash = Self::hash_url(url);
        let chunk_id = offset / self.chunk_size as u64;

        let cache_dir = self.cache_dir.join(&url_hash);
        std::fs::create_dir_all(&cache_dir)?;

        let cache_file = cache_dir.join(format!("chunk_{:016x}.bin", chunk_id));

        // Check if we need to evict
        let new_size = self.current_size.load(Ordering::Relaxed) + data.len() as u64;
        if new_size > self.max_size_bytes {
            self.evict_lru(data.len() as u64)?;
        }

        // Write to cache
        std::fs::write(&cache_file, data)?;

        // Update tracking
        self.current_size.fetch_add(data.len() as u64, Ordering::Relaxed);

        let mut log = self.access_log.lock().unwrap();
        log.push_back(CacheEntry {
            url_hash: url_hash.clone(),
            offset,
            size: data.len(),
            last_access: SystemTime::now(),
            file_path: cache_file,
        });

        Ok(())
    }

    fn evict_lru(&self, needed_bytes: u64) -> Result<()> {
        let mut log = self.access_log.lock().unwrap();
        let mut freed = 0u64;

        while freed < needed_bytes && !log.is_empty() {
            if let Some(entry) = log.pop_front() {
                // Delete cache file
                if entry.file_path.exists() {
                    std::fs::remove_file(&entry.file_path)?;
                    freed += entry.size as u64;
                    self.current_size.fetch_sub(entry.size as u64, Ordering::Relaxed);
                }
            }
        }

        Ok(())
    }

    fn hash_url(url: &Url) -> String {
        // SHA256 hash of URL
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(url.as_str().as_bytes());
        format!("{:x}", hasher.finalize())
    }
}
```

### Resume on Failure

**Problem**: Network interruptions cause download failures

**Solution**: Retry with exponential backoff

```rust
impl HttpReader {
    fn fetch_chunk_with_retry(&mut self, offset: u64, size: usize) -> Result<Vec<u8>> {
        let max_retries = 5;
        let mut retry_delay = Duration::from_secs(1);

        for attempt in 0..max_retries {
            match self.fetch_chunk(offset, size) {
                Ok(data) => return Ok(data),
                Err(e) if Self::is_retriable(&e) => {
                    eprintln!("Network error (attempt {}/{}): {}. Retrying in {:?}...",
                             attempt + 1, max_retries, e, retry_delay);
                    std::thread::sleep(retry_delay);
                    retry_delay *= 2;  // Exponential backoff
                }
                Err(e) => return Err(e),  // Non-retriable error
            }
        }

        Err(Error::MaxRetriesExceeded)
    }

    fn is_retriable(error: &Error) -> bool {
        match error {
            Error::HttpError(status) if status.is_server_error() => true,
            Error::NetworkTimeout => true,
            Error::ConnectionReset => true,
            _ => false,
        }
    }
}
```

---

## SRA Integration (Week 3-4)

### SRA Reader with Toolkit Integration

**Concept**: Use NCBI SRA Toolkit's `fasterq-dump` for streaming

```rust
pub struct SraReader {
    accession: String,
    cache: SraCache,

    // SRA Toolkit process
    process: Option<Child>,
    stdout: Option<ChildStdout>,
}

impl SraReader {
    pub fn new(accession: String, cache: SraCache) -> Result<Self> {
        // Check if SRA Toolkit is installed
        if !Self::sra_toolkit_available()? {
            return Err(Error::SraToolkitMissing);
        }

        // Check cache first
        if let Some(cached_path) = cache.get(&accession)? {
            // Already downloaded, stream from cache
            return Ok(Self {
                accession,
                cache,
                process: None,
                stdout: Some(File::open(cached_path)?.into()),
            });
        }

        // Not cached, stream via fasterq-dump
        let mut process = Command::new("fasterq-dump")
            .arg(&accession)
            .arg("--stdout")        // Stream to stdout
            .arg("--skip-technical") // Skip technical reads
            .arg("--threads").arg("4")
            .stdout(Stdio::piped())
            .spawn()?;

        let stdout = process.stdout.take()
            .ok_or(Error::SraProcessFailed)?;

        Ok(Self {
            accession,
            cache,
            process: Some(process),
            stdout: Some(stdout),
        })
    }

    fn sra_toolkit_available() -> Result<bool> {
        Ok(Command::new("fasterq-dump")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?
            .success())
    }
}

impl Read for SraReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let stdout = self.stdout.as_mut()
            .ok_or(io::Error::new(io::ErrorKind::BrokenPipe, "SRA stream closed"))?;

        stdout.read(buf)
    }
}

impl Drop for SraReader {
    fn drop(&mut self) {
        // Clean up process
        if let Some(mut process) = self.process.take() {
            let _ = process.kill();
            let _ = process.wait();
        }
    }
}
```

### SRA Cache

**Purpose**: Don't re-download SRA runs

```rust
pub struct SraCache {
    cache_dir: PathBuf,  // ~/.biofast/cache/sra/
    max_size_bytes: u64,
}

impl SraCache {
    pub fn default() -> Result<Self> {
        let cache_dir = dirs::home_dir()
            .ok_or(Error::NoCacheDir)?
            .join(".biofast")
            .join("cache")
            .join("sra");

        std::fs::create_dir_all(&cache_dir)?;

        Ok(Self {
            cache_dir,
            max_size_bytes: 100 * 1024 * 1024 * 1024,  // 100 GB for SRA
        })
    }

    pub fn get(&self, accession: &str) -> Result<Option<PathBuf>> {
        let cached = self.cache_dir.join(format!("{}.fastq", accession));

        if cached.exists() {
            Ok(Some(cached))
        } else {
            Ok(None)
        }
    }

    pub fn put(&self, accession: &str, data: impl Read) -> Result<PathBuf> {
        let dest = self.cache_dir.join(format!("{}.fastq", accession));

        let mut file = File::create(&dest)?;
        io::copy(&mut data, &mut file)?;

        Ok(dest)
    }
}
```

---

## User API

### Unified Interface

**Goal**: Identical API for local/HTTP/SRA

```rust
// Local file
let stats = FastqStream::open("data.fq.gz")?
    .gc_content()
    .compute()?;

// HTTP URL
let stats = FastqStream::open("https://example.com/data.fq.gz")?
    .gc_content()
    .compute()?;

// SRA accession
let stats = FastqStream::open("SRR1234567")?
    .gc_content()
    .compute()?;

// All three use same API!
```

### Configuration

**Cache size control**:
```rust
use biofast::config::CacheConfig;

// Set cache size budget
CacheConfig::default()
    .with_http_cache_size(50 * 1024 * 1024 * 1024)  // 50 GB
    .with_sra_cache_size(100 * 1024 * 1024 * 1024)  // 100 GB
    .apply()?;
```

**CLI flags**:
```bash
# Set cache size
biofast --cache-size 50GB gc-content https://example.com/data.fq.gz

# Clear cache
biofast --clear-cache

# Show cache usage
biofast --cache-info
```

---

## Performance Characteristics

### Bandwidth Requirements

**Scenario**: 5TB SRA dataset, 100 Mbps connection

**Traditional download**:
- Time: 5TB / 100 Mbps = 111 hours (4.6 days)
- Storage: 5TB local

**biofast streaming** (with smart cache):
- Time: Process as download proceeds (streaming)
- Storage: 50GB cache (100× less)
- Benefit: Results in ~5 days (same time), but no 5TB storage needed

**Slow connection** (10 Mbps):
- Traditional: 46 days to download
- biofast: 46 days, but progressive results (process as data arrives)

### Cache Hit Rates

**Scenario**: Re-analyzing same dataset

**First analysis**:
- Cache miss → Download chunks as needed
- Cache fills to 50GB (user budget)

**Second analysis**:
- Cache hit → No download!
- Instant processing (local file speed)

**Partial re-analysis** (same dataset, different operation):
- Cache hit for overlapping regions
- Only download new regions if needed

---

## Implementation Timeline

### Week 3 (Nov 18-24): HTTP Streaming

**Day 1-2** (Nov 18-19):
- [ ] Implement `StreamSource` abstraction
- [ ] Implement `HttpReader` with range requests
- [ ] Basic caching (no LRU yet)

**Day 3-4** (Nov 20-21):
- [ ] Implement LRU cache eviction
- [ ] Implement background prefetching
- [ ] Implement retry logic

**Day 5** (Nov 22):
- [ ] Testing (real HTTP URLs)
- [ ] Network interruption simulation
- [ ] Performance validation

### Week 4 (Nov 25-29): SRA Integration

**Day 6-7** (Nov 25-26):
- [ ] SRA Toolkit detection
- [ ] `SraReader` implementation
- [ ] SRA cache

**Day 8** (Nov 27):
- [ ] Testing (real SRA accessions)
- [ ] Large-scale validation (1M+ reads)

**Day 9-10** (Nov 28-29):
- [ ] Documentation (NETWORK_STREAMING_VISION.md)
- [ ] Examples (HTTP + SRA workflows)
- [ ] Release prep (v0.2.0)

---

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_http_reader_basic() {
    // Mock HTTP server
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET)
            .path("/data.txt")
            .header("Range", "bytes=0-1023");
        then.status(206)
            .header("Content-Range", "bytes 0-1023/10000")
            .body(&vec![b'A'; 1024]);
    });

    let url = format!("{}/data.txt", server.url());
    let mut reader = HttpReader::new(url.parse().unwrap(), HttpCache::default().unwrap()).unwrap();

    let mut buf = vec![0u8; 1024];
    assert_eq!(reader.read(&mut buf).unwrap(), 1024);
    assert_eq!(buf, vec![b'A'; 1024]);
}

#[test]
fn test_cache_eviction() {
    let cache = HttpCache::default().unwrap()
        .with_max_size(1024 * 1024);  // 1 MB cache

    let url1 = Url::parse("http://example.com/file1").unwrap();
    let url2 = Url::parse("http://example.com/file2").unwrap();

    // Fill cache
    cache.put(&url1, 0, &vec![0u8; 512 * 1024]).unwrap();  // 512 KB
    cache.put(&url1, 512 * 1024, &vec![0u8; 512 * 1024]).unwrap();  // 512 KB (total: 1 MB)

    // This should evict oldest
    cache.put(&url2, 0, &vec![0u8; 512 * 1024]).unwrap();

    // First chunk of url1 should be evicted
    assert!(cache.get(&url1, 0, 512 * 1024).unwrap().is_none());
    // Second chunk of url1 should still be there
    assert!(cache.get(&url1, 512 * 1024, 512 * 1024).unwrap().is_some());
}
```

### Integration Tests

**Real HTTP streaming**:
```rust
#[test]
#[ignore]  // Requires network
fn test_real_http_streaming() {
    // Stream from real compressed FASTQ on HTTP
    let url = "https://example.com/test.fq.gz";

    let stats = FastqStream::open(url).unwrap()
        .gc_content()
        .compute()
        .unwrap();

    assert!(stats.gc > 0.0 && stats.gc < 1.0);
}
```

**Real SRA streaming**:
```rust
#[test]
#[ignore]  // Requires SRA Toolkit + network
fn test_real_sra_streaming() {
    // Small SRA run for testing
    let accession = "SRR000001";

    let count = FastqStream::open(accession).unwrap()
        .count()
        .unwrap();

    assert!(count > 0);
}
```

---

## Success Metrics

### Technical Validation

- [ ] HTTP streaming works with real URLs
- [ ] Cache reduces bandwidth (>80% hit rate on re-analysis)
- [ ] Prefetching overlaps I/O with compute (>50% time saved)
- [ ] Resume works after network interruption
- [ ] SRA streaming works with real accessions
- [ ] Memory usage <100 MB constant (even for 5TB datasets)

### User Experience

- [ ] Transparent API (user doesn't care if local/HTTP/SRA)
- [ ] Clear progress indication (download + processing progress)
- [ ] Sensible defaults (cache size auto-detected from available disk)
- [ ] Helpful errors ("SRA Toolkit not found" → install instructions)

### Performance

- [ ] Streaming overhead <10% vs cached (with prefetching)
- [ ] Cache eviction <1 second (even for large caches)
- [ ] Network errors handled gracefully (retry succeeds)

---

## Future Enhancements (Post v1.0)

### Advanced Caching Strategies

**Predictive prefetching**:
- Analyze access patterns
- Prefetch likely-needed chunks
- Machine learning for access prediction

**Distributed caching**:
- Share cache across lab/institution
- Reduce redundant downloads
- P2P chunk sharing

### Compression-Aware Streaming

**Problem**: Can't random-access gzip files

**Solution**: Use seekable compression (bgzip, indexed gzip)
- BGZIP (block gzip) allows random access
- Only download needed blocks
- Further bandwidth reduction

### Cloud Storage Integration

**S3 streaming**:
```rust
FastqStream::open("s3://bucket/data.fq.gz")?
```

**Google Cloud Storage streaming**:
```rust
FastqStream::open("gs://bucket/data.fq.gz")?
```

---

**Last Updated**: November 3, 2025
**Status**: Design complete, implementation starts Week 3 (Nov 18)
**Target Release**: biofast v0.2.0 (November 29, 2025)

**Dependencies**: biofast v0.1.0 (core streaming must work first)
**Owner**: Scott Handley + Claude
**Impact**: Eliminates storage barrier (Data Access pillar validated)

# Load Test Results: Content-Addressable Storage Implementation

## Overview

This document summarizes the load testing performed on the Cloudflare Workers R2 storage implementation with content-addressable storage (CAS). The tests were conducted to verify the solution to R2's rate limit issues and to determine the system's performance limits.

## Test Environment

- **Platform**: Cloudflare Workers (Production)
- **URL**: https://work-rs.imjasonh.workers.dev
- **Date**: June 2025
- **Implementation**: Rust + WebAssembly with content-addressable storage

## Test Scenarios

### 1. Initial Load Test (Baseline)
Simulated the original failing scenario to verify the fix:
- **Configuration**: 5 concurrent users × 100 files each = 500 total uploads
- **File size**: Variable (test data)
- **Results**:
  - **Before CAS**: 56.5% error rate (282/500 failures)
  - **After CAS**: 0% error rate (500/500 success)
  - **Performance**: ~15 uploads/second sustained

### 2. Comprehensive Load Test
Tested various file sizes and concurrency levels:

#### Test Configuration
```
- Small files (1KB): 10 users × 50 files = 500 uploads
- Medium files (100KB): 5 users × 20 files = 100 uploads
- Large files (1MB): 3 users × 10 files = 30 uploads
- Reads: 20 users × 100 reads = 2000 reads
```

#### Results Summary
| File Size | Total Uploads | Success Rate | Avg Upload Rate |
|-----------|--------------|--------------|-----------------|
| 1KB | 500 | 100% | ~29.4/sec |
| 100KB | 100 | 100% | ~11.1/sec |
| Mixed Reads | 2000 | 100% | ~42.5/sec |

### 3. Large File Load Test
Tested the upper limits of file size handling:

#### Results by File Size
| File Size | Count | Success Rate | Avg Upload Time | Throughput |
|-----------|-------|--------------|-----------------|------------|
| 1 MB | 10 | 100% | 0.60s | 1.67 MB/s |
| 5 MB | 5 | 100% | 1.05s | 4.76 MB/s |
| 10 MB | 3 | 100% | 1.40s | 7.14 MB/s |
| 50 MB | 2 | 100% | 4.76s | 10.50 MB/s |
| 100 MB | 1 | 100% | 10.28s | 9.73 MB/s |

## Key Findings

### 1. Rate Limit Resolution
- **Problem Solved**: R2's 1 write/second/key limit no longer causes failures
- **How**: Content-addressable storage deduplicates writes
- **Result**: 0% error rate even under high concurrency

### 2. Performance Characteristics
- **Small files (≤1MB)**: Sub-second uploads, highly concurrent
- **Medium files (5-10MB)**: 1-1.5 second uploads, good concurrency
- **Large files (50-100MB)**: 5-10 second uploads, sequential recommended
- **Read performance**: Exceptional due to edge caching (42.5 reads/sec)

### 3. System Limits
- **Maximum file size**: 100MB (Cloudflare Workers request body limit)
- **Concurrent performance**: No degradation up to 20 concurrent operations
- **Throughput**: Consistent 7-10 MB/s for large files
- **Deduplication**: Working correctly, duplicate content uploads return quickly

## Test Methodology

### Load Test Script Structure
The tests used bash scripts with the following approach:

1. **File Generation**:
   ```bash
   # Small files: Random data using head/dev/urandom
   # Large files: Using dd to create binary files
   dd if=/dev/urandom of="$temp_file" bs=1048576 count=$size_mb
   ```

2. **Upload Pattern**:
   ```bash
   curl -X PUT "$BASE_URL/files/$filename" \
     -H "Content-Type: application/octet-stream" \
     --data-binary "@$temp_file" \
     --max-time $timeout
   ```

3. **Concurrency**:
   - Used bash background jobs (`&`) for parallel uploads
   - Synchronized with `wait` command
   - Tracked results in CSV format

4. **Metrics Collection**:
   - HTTP status codes
   - Upload duration
   - Success/failure counts
   - Throughput calculations

### Content Patterns
- **Unique content**: 70-80% to simulate real usage
- **Duplicate content**: 20-30% to test deduplication
- **File naming**: Sequential with user/file identifiers

## Conclusions

1. **CAS Implementation Success**: Completely eliminated R2 rate limit errors
2. **Scalability Proven**: Handles high concurrency without degradation
3. **Large File Support**: Successfully handles files up to 100MB limit
4. **Production Ready**: 100% success rate across all test scenarios

## Recommendations

1. **Document Size Limits**: Clearly state 100MB maximum file size
2. **Optimize for Common Cases**: Most files are <10MB, already well optimized
3. **Monitor Deduplication Ratio**: Track storage savings from CAS
4. **Consider Multipart Upload**: For potential future support of >100MB files

## Future Testing

To reproduce these tests:

1. Create bash scripts following the patterns above
2. Use similar file size distributions
3. Test during different load conditions
4. Monitor both success rates and response times
5. Track deduplication effectiveness with identical content tests

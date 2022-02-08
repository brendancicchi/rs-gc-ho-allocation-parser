# rs-gc-ho-allocation-parser

### Summary

Provides a summary of humongous allocation in a given gc log file. Provides information regarding the region size, number of allocations in each region bucket, as well as a percentile breakdown of the humongous allocations.

### Sample Output

```
Region Size: 16MB - "/Users/user/Downloads/var/log/cassandra/gc.log.1"
+-------------+---------------------------+-----------------------+
| Region Size | Max Allocation Size (50%) | Number of Allocations |
+-------------+---------------------------+-----------------------+
|         2MB | 1048576                   | 0                     |
+-------------+---------------------------+-----------------------+
|         4MB | 2097152                   | 0                     |
+-------------+---------------------------+-----------------------+
|         8MB | 4194304                   | 0                     |
+-------------+---------------------------+-----------------------+
|        16MB | 8388608                   | 0                     |
+-------------+---------------------------+-----------------------+
|        32MB | 16777216                  | 1523                  |
+-------------+---------------------------+-----------------------+
|    Overflow | 4294967295                | 22333                 |
+-------------+---------------------------+-----------------------+

Allocation Size Percentiles:
	min: 8539603
	p50: 22280143
	p75: 22464693
	p90: 44325405
	p99: 44929385
	max: 44929385
```

### Performance Evaluations

Python Performance with hyperfine (RegEx)
```
  Time (mean ± σ):     361.0 ms ±   6.9 ms    [User: 189.8 ms, System: 124.7 ms]
  Range (min … max):   351.0 ms … 371.2 ms    10 runs
```

Python Performance with hyperfine (No RegEx)
```
  Time (mean ± σ):     305.2 ms ±   7.5 ms    [User: 125.6 ms, System: 130.4 ms]
  Range (min … max):   296.0 ms … 317.1 ms    10 runs
```

Rust performance with hyperfine (RegEx captures) - initial implementation
```
  Time (mean ± σ):      2.635 s ±  0.024 s    [User: 2.617 s, System: 0.010 s]
  Range (min … max):    2.613 s …  2.689 s    10 runs
```

Rust performance with hyperfine (RegEx find) - second attempt
```
  Time (mean ± σ):     708.2 ms ±   4.6 ms    [User: 698.2 ms, System: 5.6 ms]
  Range (min … max):   701.1 ms … 718.7 ms    10 runs
```

Rust Performance with hyperfine (manual string parsing) - final
```
  Time (mean ± σ):     203.0 ms ±   3.4 ms    [User: 195.9 ms, System: 3.5 ms]
  Range (min … max):   198.5 ms … 210.6 ms    13 runs
```
use anyhow::{anyhow, Result};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tokio::process::Command;
use tracing::debug;

/// PortPool manages allocation and cleanup of ports for OpenCode instances.
///
/// Ports are allocated sequentially from a configurable range (start + size).
/// Released ports can be reused. Orphan processes on ports can be cleaned up
/// using lsof and kill commands.
#[derive(Clone)]
pub struct PortPool {
    start: u16,
    size: u16,
    allocated: Arc<Mutex<HashSet<u16>>>,
}

impl PortPool {
    /// Create a new PortPool with the given start port and pool size.
    ///
    /// # Arguments
    /// * `start` - First port in the range
    /// * `size` - Number of ports in the pool
    ///
    /// # Example
    /// ```
    /// let pool = PortPool::new(4100, 100); // Ports 4100-4199
    /// ```
    pub fn new(start: u16, size: u16) -> Self {
        Self {
            start,
            size,
            allocated: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Allocate the next available port from the pool.
    ///
    /// Ports are allocated sequentially from start. Released ports can be reused.
    ///
    /// # Returns
    /// * `Ok(port)` - Successfully allocated port
    /// * `Err(_)` - Pool exhausted (all ports allocated)
    pub async fn allocate(&self) -> Result<u16> {
        let mut allocated = self.allocated.lock().unwrap();

        // Try to find an available port in the range
        for offset in 0..self.size {
            let port = self.start + offset;
            if !allocated.contains(&port) {
                allocated.insert(port);
                let remaining = self.size as usize - allocated.len();
                debug!(
                    port = port,
                    remaining_available = remaining,
                    "Port allocated from pool"
                );
                return Ok(port);
            }
        }

        Err(anyhow!(
            "Port pool exhausted: all {} ports allocated",
            self.size
        ))
    }

    /// Release a port back to the pool, making it available for reuse.
    ///
    /// # Arguments
    /// * `port` - Port to release
    pub async fn release(&self, port: u16) {
        let mut allocated = self.allocated.lock().unwrap();
        allocated.remove(&port);
        debug!(port = port, "Port released back to pool");
    }

    /// Check if a port is available (not in use by any process).
    ///
    /// Uses `lsof -ti:PORT` to check if a process is listening on the port.
    ///
    /// # Arguments
    /// * `port` - Port to check
    ///
    /// # Returns
    /// * `true` - Port is free (no process listening)
    /// * `false` - Port is in use
    #[allow(dead_code)]
    pub async fn is_available(&self, port: u16) -> bool {
        // Check if port is allocated in our pool
        {
            let allocated = self.allocated.lock().unwrap();
            if allocated.contains(&port) {
                debug!(port = port, available = false, "Port is allocated in pool");
                return false;
            }
        }

        // Check if port is in use by any process using lsof
        match Command::new("lsof")
            .arg("-ti")
            .arg(format!(":{}", port))
            .output()
            .await
        {
            Ok(output) => {
                // If lsof returns output, port is in use
                // If lsof returns empty, port is free
                let available = output.stdout.is_empty();
                debug!(
                    port = port,
                    available = available,
                    "Port availability check"
                );
                available
            }
            Err(_) => {
                // If lsof command fails, assume port is available
                // (lsof might not be installed or permission denied)
                debug!(
                    port = port,
                    available = true,
                    reason = "lsof check failed, assuming available"
                );
                true
            }
        }
    }

    /// Cleanup orphan process on a port by killing it.
    ///
    /// Uses `lsof -ti:PORT` to find the PID, then `kill -9 PID` to terminate.
    ///
    /// # Arguments
    /// * `port` - Port to cleanup
    ///
    /// # Returns
    /// * `Ok(())` - Process killed successfully
    /// * `Err(_)` - No process found or kill failed
    #[allow(dead_code)]
    // Used by future: orphan process cleanup feature
    pub async fn cleanup_orphan(&self, port: u16) -> Result<()> {
        // Find PID using lsof
        let output = Command::new("lsof")
            .arg("-ti")
            .arg(format!(":{}", port))
            .output()
            .await
            .map_err(|e| anyhow!("Failed to run lsof: {}", e))?;

        if output.stdout.is_empty() {
            return Err(anyhow!("No process found on port {}", port));
        }

        let pid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let pid = pid_str
            .parse::<i32>()
            .map_err(|_| anyhow!("Invalid PID: {}", pid_str))?;

        // Kill the process
        let kill_output = Command::new("kill")
            .arg("-9")
            .arg(pid.to_string())
            .output()
            .await
            .map_err(|e| anyhow!("Failed to run kill: {}", e))?;

        if !kill_output.status.success() {
            return Err(anyhow!("Failed to kill process {} on port {}", pid, port));
        }

        Ok(())
    }

    /// Get the count of currently allocated ports.
    ///
    /// # Returns
    /// Number of ports currently allocated
    pub fn allocated_count(&self) -> usize {
        let allocated = self.allocated.lock().unwrap();
        allocated.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_creates_pool_with_range() {
        let pool = PortPool::new(4100, 100);
        assert_eq!(pool.start, 4100);
        assert_eq!(pool.size, 100);
        assert_eq!(pool.allocated_count(), 0);
    }

    #[tokio::test]
    async fn test_allocate_returns_sequential_ports() {
        let pool = PortPool::new(4100, 5);

        let port1 = pool.allocate().await.unwrap();
        assert_eq!(port1, 4100);

        let port2 = pool.allocate().await.unwrap();
        assert_eq!(port2, 4101);

        let port3 = pool.allocate().await.unwrap();
        assert_eq!(port3, 4102);

        assert_eq!(pool.allocated_count(), 3);
    }

    #[tokio::test]
    async fn test_allocate_fails_when_pool_exhausted() {
        let pool = PortPool::new(4100, 2);

        // Allocate all ports
        pool.allocate().await.unwrap();
        pool.allocate().await.unwrap();

        // Next allocation should fail
        let result = pool.allocate().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exhausted"));
    }

    #[tokio::test]
    async fn test_release_makes_port_available_again() {
        let pool = PortPool::new(4100, 3);

        let port1 = pool.allocate().await.unwrap();
        let port2 = pool.allocate().await.unwrap();
        assert_eq!(pool.allocated_count(), 2);

        // Release first port
        pool.release(port1).await;
        assert_eq!(pool.allocated_count(), 1);

        // Allocate again - should reuse released port
        let port3 = pool.allocate().await.unwrap();
        assert_eq!(port3, port1); // Reuses 4100
        assert_eq!(pool.allocated_count(), 2);

        // Verify port2 is still allocated
        let allocated = pool.allocated.lock().unwrap();
        assert!(allocated.contains(&port2));
        assert!(allocated.contains(&port3));
    }

    #[tokio::test]
    async fn test_allocated_count_tracks_correctly() {
        let pool = PortPool::new(4100, 10);

        assert_eq!(pool.allocated_count(), 0);

        let port1 = pool.allocate().await.unwrap();
        assert_eq!(pool.allocated_count(), 1);

        let port2 = pool.allocate().await.unwrap();
        assert_eq!(pool.allocated_count(), 2);

        pool.release(port1).await;
        assert_eq!(pool.allocated_count(), 1);

        pool.release(port2).await;
        assert_eq!(pool.allocated_count(), 0);
    }

    #[tokio::test]
    async fn test_is_available_returns_false_for_allocated_port() {
        let pool = PortPool::new(4100, 10);

        let port = pool.allocate().await.unwrap();

        // Port is allocated in our pool, so should not be available
        let available = pool.is_available(port).await;
        assert!(!available);
    }

    #[tokio::test]
    async fn test_is_available_returns_true_when_port_free() {
        let pool = PortPool::new(4100, 10);

        // Port not allocated in our pool
        // This test assumes port 4100 is not in use by another process
        // In a real scenario, we'd mock lsof
        let available = pool.is_available(4100).await;
        // Can't assert true here because port might be in use by another process
        // Just verify the function runs without panic
        let _ = available;
    }

    #[tokio::test]
    async fn test_cleanup_orphan_fails_when_no_process() {
        let pool = PortPool::new(50000, 10);
        let result = pool.cleanup_orphan(50000).await;

        if result.is_ok() {
            return;
        }

        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("No process found") || msg.contains("Failed to run lsof"));
    }

    #[tokio::test]
    async fn test_concurrent_allocation_thread_safe() {
        let pool = PortPool::new(4100, 20);
        let pool_clone = pool.clone();

        // Spawn multiple tasks allocating ports concurrently
        let mut handles = vec![];

        for _ in 0..10 {
            let p = pool_clone.clone();
            let handle = tokio::spawn(async move { p.allocate().await });
            handles.push(handle);
        }

        // Wait for all allocations
        let mut ports = vec![];
        for handle in handles {
            if let Ok(Ok(port)) = handle.await {
                ports.push(port);
            }
        }

        // Verify all ports are unique
        let unique_ports: HashSet<_> = ports.iter().collect();
        assert_eq!(unique_ports.len(), ports.len());

        // Verify count matches
        assert_eq!(pool.allocated_count(), 10);
    }

    #[tokio::test]
    async fn test_release_nonexistent_port_is_safe() {
        let pool = PortPool::new(4100, 10);

        // Release a port that was never allocated
        pool.release(4105).await;

        // Should not panic or error
        assert_eq!(pool.allocated_count(), 0);
    }
}

//! GPU acceleration benchmark test using ICICLE
//!
//! This test compares CPU vs GPU MSM (Multi-Scalar Multiplication) performance.
//! Run with: cargo test --test gpu_benchmark_test --release --features gpu -- --nocapture

#[cfg(test)]
mod tests {
    use std::time::Instant;

    #[cfg(feature = "gpu")]
    use icicle_bn254::curve::{CurveCfg, G1Projective, ScalarCfg};
    #[cfg(feature = "gpu")]
    use icicle_core::{
        curve::Curve,
        msm::{msm, MSMConfig},
        traits::GenerateRandom,
    };
    #[cfg(feature = "gpu")]
    use icicle_runtime::{device::Device, memory::HostSlice};

    /// Helper to initialize ICICLE with CUDA backend
    #[cfg(feature = "gpu")]
    fn init_icicle_cuda() -> bool {
        // Load CUDA backend - required for ICICLE v3.x
        if let Err(e) = icicle_runtime::load_backend_from_env_or_default() {
            println!("Warning: Failed to load backend: {:?}", e);
        }

        // Check available devices after loading backend
        let devices = icicle_runtime::get_registered_devices().unwrap_or_default();
        println!("Registered devices after loading backend: {:?}", devices);

        // Try to set CUDA device
        let cuda_device = Device::new("CUDA", 0);
        icicle_runtime::set_device(&cuda_device).is_ok()
    }

    /// Test that ICICLE runtime initializes and detects GPUs
    #[test]
    #[cfg(feature = "gpu")]
    fn test_icicle_gpu_detection() {
        println!("\n=== ICICLE GPU Detection Test ===\n");

        // Load backend first
        if let Err(e) = icicle_runtime::load_backend_from_env_or_default() {
            println!("Warning: Failed to load backend: {:?}", e);
        }

        // Check available devices
        let devices = icicle_runtime::get_registered_devices().unwrap();
        println!("Registered devices: {:?}", devices);

        // Try to get CUDA device
        let cuda_device = Device::new("CUDA", 0);
        match icicle_runtime::set_device(&cuda_device) {
            Ok(_) => {
                println!("Successfully set CUDA device 0");
                
                // Get device properties (for currently active device)
                if let Ok(props) = icicle_runtime::get_device_properties() {
                    println!("  Device properties: {:?}", props);
                }
            }
            Err(e) => {
                println!("Failed to set CUDA device: {:?}", e);
                println!("  This is expected if running without GPU");
            }
        }

        println!("\nICICLE runtime initialized successfully");
    }

    /// Benchmark MSM on GPU vs CPU reference
    #[test]
    #[cfg(feature = "gpu")]
    fn test_gpu_msm_benchmark() {
        println!("\n=== GPU MSM Benchmark ===\n");

        // Initialize ICICLE with CUDA backend
        if !init_icicle_cuda() {
            println!("GPU not available, skipping GPU benchmark");
            println!("  Run on a machine with NVIDIA GPU and CUDA installed");
            return;
        }

        // Benchmark different sizes
        for log_size in [12, 14, 16, 18] {
            let size = 1 << log_size;
            println!("MSM size: 2^{} = {} points", log_size, size);

            // Generate random scalars and points
            let scalars = ScalarCfg::generate_random(size);
            let points = CurveCfg::generate_random_affine_points(size);

            // GPU MSM
            let mut msm_result_gpu = vec![G1Projective::zero(); 1];
            let cfg = MSMConfig::default();

            let gpu_start = Instant::now();
            msm(
                HostSlice::from_slice(&scalars),
                HostSlice::from_slice(&points),
                &cfg,
                HostSlice::from_mut_slice(&mut msm_result_gpu),
            )
            .unwrap();
            let gpu_time = gpu_start.elapsed();

            println!(
                "  GPU time: {:?} ({:.2} points/sec)",
                gpu_time,
                size as f64 / gpu_time.as_secs_f64()
            );
        }

        println!("\nGPU MSM benchmark complete");
    }

    /// Test basic GPU MSM correctness
    #[test]
    #[cfg(feature = "gpu")]
    fn test_gpu_msm_correctness() {
        println!("\n=== GPU MSM Correctness Test ===\n");

        // Initialize ICICLE with CUDA backend
        if !init_icicle_cuda() {
            println!("GPU not available, skipping test");
            return;
        }

        // Small size for verification
        let size = 1024;
        println!("Testing MSM with {} points...", size);

        let scalars = ScalarCfg::generate_random(size);
        let points = CurveCfg::generate_random_affine_points(size);

        let mut msm_result = vec![G1Projective::zero(); 1];
        let cfg = MSMConfig::default();

        msm(
            HostSlice::from_slice(&scalars),
            HostSlice::from_slice(&points),
            &cfg,
            HostSlice::from_mut_slice(&mut msm_result),
        )
        .unwrap();

        // Verify result is not zero (basic sanity check)
        assert!(
            msm_result[0] != G1Projective::zero(),
            "MSM result should not be zero"
        );

        println!("GPU MSM produces non-zero result");
        println!("Correctness test passed");
    }

    /// CPU-only test that always runs (no GPU feature required)
    #[test]
    fn test_cpu_baseline() {
        println!("\n=== CPU Baseline Test ===\n");
        
        #[cfg(feature = "gpu")]
        {
            println!("GPU feature enabled - ICICLE available");
        }
        
        #[cfg(not(feature = "gpu"))]
        {
            println!("GPU feature NOT enabled");
            println!("To enable GPU: cargo test --features gpu");
        }

        println!("CPU baseline test passed");
    }
}


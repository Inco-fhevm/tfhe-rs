use crate::high_level_api::details::MaybeCloned;
#[cfg(feature = "gpu")]
use crate::high_level_api::global_state::{self, with_thread_local_cuda_stream};
#[cfg(feature = "gpu")]
use crate::integer::gpu::ciphertext::CudaIntegerRadixCiphertext;
#[cfg(feature = "gpu")]
use crate::integer::gpu::ciphertext::CudaSignedRadixCiphertext;
use crate::Device;
use serde::{Deserializer, Serializer};

pub(crate) enum RadixCiphertext {
    Cpu(crate::integer::SignedRadixCiphertext),
    #[cfg(feature = "gpu")]
    Cuda(CudaSignedRadixCiphertext),
}

impl From<crate::integer::SignedRadixCiphertext> for RadixCiphertext {
    fn from(value: crate::integer::SignedRadixCiphertext) -> Self {
        Self::Cpu(value)
    }
}

#[cfg(feature = "gpu")]
impl From<CudaSignedRadixCiphertext> for RadixCiphertext {
    fn from(value: CudaSignedRadixCiphertext) -> Self {
        Self::Cuda(value)
    }
}

impl Clone for RadixCiphertext {
    fn clone(&self) -> Self {
        match self {
            Self::Cpu(inner) => Self::Cpu(inner.clone()),
            #[cfg(feature = "gpu")]
            Self::Cuda(inner) => with_thread_local_cuda_stream(|stream| {
                let inner = inner.duplicate(stream);
                Self::Cuda(inner)
            }),
        }
    }
}

impl serde::Serialize for RadixCiphertext {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.on_cpu().serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for RadixCiphertext {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut deserialized = Self::Cpu(crate::integer::SignedRadixCiphertext::deserialize(
            deserializer,
        )?);
        deserialized.move_to_device_of_server_key_if_set();
        Ok(deserialized)
    }
}

impl RadixCiphertext {
    pub(crate) fn current_device(&self) -> Device {
        match self {
            Self::Cpu(_) => Device::Cpu,
            #[cfg(feature = "gpu")]
            Self::Cuda(_) => Device::CudaGpu,
        }
    }

    /// Returns the a ref to the inner cpu ciphertext if self is on the CPU, otherwise, returns a
    /// copy that is on the CPU
    pub(crate) fn on_cpu(&self) -> MaybeCloned<'_, crate::integer::SignedRadixCiphertext> {
        match self {
            Self::Cpu(ct) => MaybeCloned::Borrowed(ct),
            #[cfg(feature = "gpu")]
            Self::Cuda(ct) => with_thread_local_cuda_stream(|stream| {
                let cpu_ct = ct.to_signed_radix_ciphertext(stream);
                MaybeCloned::Cloned(cpu_ct)
            }),
        }
    }

    /// Returns the inner cpu ciphertext if self is on the CPU, otherwise, returns a copy
    /// that is on the CPU
    #[cfg(feature = "gpu")]
    pub(crate) fn on_gpu(&self) -> MaybeCloned<'_, CudaSignedRadixCiphertext> {
        match self {
            Self::Cpu(ct) => with_thread_local_cuda_stream(|stream| {
                let ct = CudaSignedRadixCiphertext::from_signed_radix_ciphertext(ct, stream);
                MaybeCloned::Cloned(ct)
            }),
            #[cfg(feature = "gpu")]
            Self::Cuda(ct) => MaybeCloned::Borrowed(ct),
        }
    }

    pub(crate) fn as_cpu_mut(&mut self) -> &mut crate::integer::SignedRadixCiphertext {
        match self {
            Self::Cpu(radix_ct) => radix_ct,
            #[cfg(feature = "gpu")]
            _ => {
                self.move_to_device(Device::Cpu);
                self.as_cpu_mut()
            }
        }
    }

    #[cfg(feature = "gpu")]
    pub(crate) fn as_gpu_mut(&mut self) -> &mut CudaSignedRadixCiphertext {
        if let Self::Cuda(radix_ct) = self {
            radix_ct
        } else {
            self.move_to_device(Device::CudaGpu);
            self.as_gpu_mut()
        }
    }

    pub(crate) fn into_cpu(self) -> crate::integer::SignedRadixCiphertext {
        match self {
            Self::Cpu(cpu_ct) => cpu_ct,
            #[cfg(feature = "gpu")]
            Self::Cuda(ct) => {
                with_thread_local_cuda_stream(|stream| ct.to_signed_radix_ciphertext(stream))
            }
        }
    }

    #[allow(unused)]
    #[cfg(feature = "gpu")]
    pub(crate) fn into_gpu(self) -> CudaSignedRadixCiphertext {
        match self {
            Self::Cpu(cpu_ct) => with_thread_local_cuda_stream(|stream| {
                CudaSignedRadixCiphertext::from_signed_radix_ciphertext(&cpu_ct, stream)
            }),
            Self::Cuda(ct) => ct,
        }
    }

    pub(crate) fn move_to_device(&mut self, device: Device) {
        match (&self, device) {
            (Self::Cpu(_), Device::Cpu) => {
                // Nothing to do, we already are on the correct device
            }
            #[cfg(feature = "gpu")]
            (Self::Cuda(_), Device::CudaGpu) => {
                // Nothing to do, we already are on the correct device
            }
            #[cfg(feature = "gpu")]
            (Self::Cpu(ct), Device::CudaGpu) => {
                let new_inner = with_thread_local_cuda_stream(|stream| {
                    CudaSignedRadixCiphertext::from_signed_radix_ciphertext(ct, stream)
                });
                *self = Self::Cuda(new_inner);
            }
            #[cfg(feature = "gpu")]
            (Self::Cuda(ct), Device::Cpu) => {
                let new_inner =
                    with_thread_local_cuda_stream(|stream| ct.to_signed_radix_ciphertext(stream));
                *self = Self::Cpu(new_inner);
            }
        }
    }

    #[inline]
    #[allow(clippy::unused_self)]
    pub(crate) fn move_to_device_of_server_key_if_set(&mut self) {
        #[cfg(feature = "gpu")]
        if let Some(device) = global_state::device_of_internal_keys() {
            self.move_to_device(device);
        }
    }
}

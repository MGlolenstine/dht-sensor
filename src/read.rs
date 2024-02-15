use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal_async::delay::DelayNs;

const TIMEOUT_US: u8 = 100;

#[derive(Debug)]
pub enum DhtError<E> {
    PinError(E),
    ChecksumMismatch,
    Timeout,
}

impl<E> From<E> for DhtError<E> {
    fn from(error: E) -> DhtError<E> {
        DhtError::PinError(error)
    }
}

pub trait InputOutputPin<E>: InputPin<Error = E> + OutputPin<Error = E> {}
impl<T, E> InputOutputPin<E> for T where T: InputPin<Error = E> + OutputPin<Error = E> {}

async fn read_bit<E>(
    delay: &mut impl DelayNs,
    pin: &mut impl InputPin<Error = E>,
) -> Result<bool, DhtError<E>> {
    wait_until_timeout(delay, || pin.is_high()).await?;
    delay.delay_us(35).await;
    let high = pin.is_high()?;
    wait_until_timeout(delay, || pin.is_low()).await?;
    Ok(high)
}

async fn read_byte<E>(
    delay: &mut impl DelayNs,
    pin: &mut impl InputPin<Error = E>,
) -> Result<u8, DhtError<E>> {
    let mut byte: u8 = 0;
    for i in 0..8 {
        let bit_mask = 1 << (7 - (i % 8));
        if read_bit(delay, pin).await? {
            byte |= bit_mask;
        }
    }
    Ok(byte)
}

pub async fn read_raw<E, D: DelayNs>(
    delay: &mut D,
    pin: &mut impl InputOutputPin<E>,
) -> Result<[u8; 4], DhtError<E>> {
    pin.set_high().ok();
    delay.delay_us(48).await;

    wait_until_timeout(delay, || pin.is_high()).await?;
    wait_until_timeout(delay, || pin.is_low()).await?;

    let mut data = [0; 4];
    for b in data.iter_mut() {
        *b = read_byte(delay, pin).await?;
    }
    let checksum = read_byte(delay, pin).await?;
    if data.iter().fold(0u8, |sum, v| sum.wrapping_add(*v)) != checksum {
        Err(DhtError::ChecksumMismatch)
    } else {
        Ok(data)
    }
}

/// Wait until the given function returns true or the timeout is reached.
async fn wait_until_timeout<D: DelayNs, E, F>(delay: &mut D, mut func: F) -> Result<(), DhtError<E>>
where
    F: FnMut() -> Result<bool, E>,
{
    for _ in 0..TIMEOUT_US {
        if func()? {
            return Ok(());
        }
        delay.delay_us(1).await;
    }
    Err(DhtError::Timeout)
}

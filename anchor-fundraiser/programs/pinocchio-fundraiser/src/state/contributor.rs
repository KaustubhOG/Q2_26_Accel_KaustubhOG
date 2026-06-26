use pinocchio::account::AccountView;

pub const CONTRIBUTOR_SIZE: usize = 1 + 8;
pub const CONTRIBUTOR_DISCRIMINATOR: u8 = 1;

pub struct Contributor;

impl Contributor {
    pub const SIZE: usize = CONTRIBUTOR_SIZE;

    #[inline(always)]
    pub fn discriminator(account: &AccountView) -> u8 {
        unsafe { *account.data_ptr() }
    }

    #[inline(always)]
    pub fn amount(account: &AccountView) -> u64 {
        unsafe { u64::from_le_bytes(*(account.data_ptr().add(1) as *const [u8; 8])) }
    }

    #[inline(always)]
    pub fn write(account: &AccountView, amount: u64) {
        let ptr = account.data_ptr();
        unsafe {
            *ptr = CONTRIBUTOR_DISCRIMINATOR;
            core::ptr::copy_nonoverlapping(amount.to_le_bytes().as_ptr(), ptr.add(1), 8);
        }
    }

    #[inline(always)]
    pub fn set_amount(account: &AccountView, amount: u64) {
        let ptr = account.data_ptr();
        unsafe {
            core::ptr::copy_nonoverlapping(amount.to_le_bytes().as_ptr(), ptr.add(1), 8);
        }
    }
}

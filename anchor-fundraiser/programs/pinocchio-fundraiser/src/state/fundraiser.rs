use pinocchio::account::AccountView;

pub const FUNDRAISER_SIZE: usize = 1 + 32 + 32 + 8 + 8 + 8 + 1 + 1;
pub const FUNDRAISER_DISCRIMINATOR: u8 = 0;

pub struct Fundraiser;

impl Fundraiser {
    pub const SIZE: usize = FUNDRAISER_SIZE;

    #[inline(always)]
    pub fn discriminator(account: &AccountView) -> u8 {
        unsafe { *account.data_ptr() }
    }

    #[inline(always)]
    pub fn maker(account: &AccountView) -> &[u8; 32] {
        unsafe { &*(account.data_ptr().add(1) as *const [u8; 32]) }
    }

    #[inline(always)]
    pub fn mint_to_raise(account: &AccountView) -> &[u8; 32] {
        unsafe { &*(account.data_ptr().add(33) as *const [u8; 32]) }
    }

    #[inline(always)]
    pub fn amount_to_raise(account: &AccountView) -> u64 {
        unsafe { u64::from_le_bytes(*(account.data_ptr().add(65) as *const [u8; 8])) }
    }

    #[inline(always)]
    pub fn current_amount(account: &AccountView) -> u64 {
        unsafe { u64::from_le_bytes(*(account.data_ptr().add(73) as *const [u8; 8])) }
    }

    #[inline(always)]
    pub fn time_started(account: &AccountView) -> i64 {
        unsafe { i64::from_le_bytes(*(account.data_ptr().add(81) as *const [u8; 8])) }
    }

    #[inline(always)]
    pub fn duration(account: &AccountView) -> u8 {
        unsafe { *account.data_ptr().add(89) }
    }

    #[inline(always)]
    pub fn bump(account: &AccountView) -> u8 {
        unsafe { *account.data_ptr().add(90) }
    }

    #[inline(always)]
    pub fn write(
        account: &AccountView,
        maker: &[u8; 32],
        mint_to_raise: &[u8; 32],
        amount_to_raise: u64,
        current_amount: u64,
        time_started: i64,
        duration: u8,
        bump: u8,
    ) {
        let ptr = account.data_ptr();
        unsafe {
            *ptr = FUNDRAISER_DISCRIMINATOR;
            core::ptr::copy_nonoverlapping(maker.as_ptr(), ptr.add(1), 32);
            core::ptr::copy_nonoverlapping(mint_to_raise.as_ptr(), ptr.add(33), 32);
            core::ptr::copy_nonoverlapping(amount_to_raise.to_le_bytes().as_ptr(), ptr.add(65), 8);
            core::ptr::copy_nonoverlapping(current_amount.to_le_bytes().as_ptr(), ptr.add(73), 8);
            core::ptr::copy_nonoverlapping(time_started.to_le_bytes().as_ptr(), ptr.add(81), 8);
            *ptr.add(89) = duration;
            *ptr.add(90) = bump;
        }
    }

    #[inline(always)]
    pub fn set_current_amount(account: &AccountView, amount: u64) {
        let ptr = account.data_ptr();
        unsafe {
            core::ptr::copy_nonoverlapping(amount.to_le_bytes().as_ptr(), ptr.add(73), 8);
        }
    }
}

#[macro_export]
/// This macro logs during test mode and returns early
/// useful for debugging.
macro_rules! fail {
    ($e:expr) => {
        #[cfg(test)]
        msg!("Error {:?}", $e);
        return Err($e.into());
    };
}

#[macro_export]
macro_rules! some_or_continue {
    ($res:expr, $loop:lifetime) => {
        match $res {
            Some(val) => val,
            None => {
                // warn!("An error: {}; skipped.", e);
                continue $loop;
            }
        }
    };
}

/// Extract a key from a ProgramAccount or Context
#[macro_export]
macro_rules! key {
    ($account:ident) => {
        *$account.to_account_info().key
    };
    ($ctx:ident, $account:ident) => {
        *$ctx.accounts.$account.to_account_info().key
    };
}

#[macro_export]
macro_rules! log_compute_units {
    ($prefix: literal) => {
        msg!($prefix);
        #[cfg(not(test))]
        sol_log_compute_units();
    };
    ($prefix: literal, $arg:tt) => {
        msg!($prefix, $arg);
        #[cfg(not(test))]
        sol_log_compute_units();
    };
}

# jouet-paiement
### Table of Contents
- [How to run it?](#how-to-run-it)
- [Assumptions](#assumptions)
- [Answers to some of the questions in the sheet](#answers-to-some-of-the-questions-in-the-sheet)
- [Limits](#limits)
- [Appendix](#appendix)


# How to run it?
### Happy path:
```shell
$ cargo run -- tests/small_input.txt > accounts.csv
```
### An erroneous transaction sequence:
(Also available as [e2e_small_input_with_transaction_process_error_using_async_processor](tests/e2e.rs))
```shell
$ cargo run -- tests/small_input_with_transaction_process_error.txt
```


# Assumptions
1. Can a withdrawal (debit) operation be disputed?\
    I assumed not. But I did not find specific answer in the question sheet.
    If withdrawals are disputable, it would then look like:
    ```
    type,       transaction_id, amount
    deposit,                 1,   5.00
    withdrawal,              2,   4.00
    dispute,                 2,
    ```
    Here's the account state change sequence:
    ```
    time,                 available,  held
    t0,                        0.00,  0.00
    after_transaction_1,       5.00,  0.00
    after_transaction_2,       1.00,  0.00
    after_transaction_3,       5.00, -4.00
    ```
    The amount disputed is `-4.00` because it is a withdrawal. The available
    amount *decreases* by a negative amount, means its absolute value
    increases. This is also consistent with temporary rolling back the
    withdrawal transaction. Same goes with the held amount: it *increases* by
    a negative amount, from `0.00` to `-4.00`. Weird ...\
    If withdrawals are not disputable, then they are not resolvable nor back
    chargable either.\
    I started with allowing the dispute of a withdrawal, but that got very
    messy and I decided to change course and follow my gut feeling that
    withdrawals are not disputable. I wanted to provide a separate
    implementation that actually supports disputing withdrawal but I guess I am
    running out of time so in the submitted work, withdrawals are **not**
    disputable.
    You can find the test cases in appendix.
1. Can an account available go negative? (a potentially missing `Pending` status
of deposit transactions)\
    Consider following transaction sequence for a client: \
    ```
    type,       transaction_id, amount
    deposit,                 1,   5.00
    withdrawal,              2,   4.00
    dispute,                 3,
    ```
    Here's the account state change sequence:
    ```
    time,                 available, held
    t0,                        0.00, 0.00
    after_transaction_1,       5.00, 0.00
    after_transaction_2,       1.00, 0.00
    after_transaction_3,      -4.00, 5.00
    ```
    This does not make much sense but it does comply with the requirement given
    in the question sheet. I do think that in real life, for every deposit
    arriving at an account, it will firstly be `Pending`, and will **not** be
    included in the available amount. A subsequent opereation / transaction
    named "Clear" or "Settle" will then turn the `Pending` deposit into
    something like `Accepted`, then this amount of fund will be included in
    available amount. And only `Pending` deposits can be disputed. Given that
    this is not stated in the requirement, I deliberately omitted them and just
    implemented exactly what the requirement says, which results in the
    possibility of the available amount being negative.
1. Error handling\
    (see test cases at [ErrorHandler](src/transaction_stream_processor/error_handler.rs))
    - The transaction process on an account could fail in following 4 cases:
        - AccountLocked (fails the process) \
            No transaction (\* see idempotency section) can be applied to a
            locked account. If that happens, the process of the file fails.
        - IncompatibleTransaction (fails the process) \
            Cases like, a "resolve" is applied to a non disputed deposit,
            indicate a potential severe issue (could be records getting out of
            order by the requirement says that the records are already ordered
            chronologically), so that process of the file fails.
        - InsufficientFundForWithdrawal (suppressed) \
            The account will not be updated on such cases and the process of
            the file continues.
        - NoTransactionFound (suppressed) \
            As stated in the requirement that this can be ignored.
    - Idempotency \
        Although the requirement says that transaction id is globally unique,
        I think it is a good practice to have built-in idempotency to suit
        cases like the following example:
        ```
        type,    client, tx, amount
        deposit,      1, 10,    4.0
        deposit,      1, 10,    4.0
        ```
        On the other hand, for a locked account, I am allowing cases like this:
        ```
        type,       client, tx, amount
        deposit,         2,  1,    3.0
        deposit,         2,  2,    2.0
        dispute,         2,  2,
        chargeback,      2,  2,
        dispute,         2,  2,
        ```
        After the chargeback, the account would have been locked. Subsequent
        transactions should all be rejected. However, if the process of a file
        fails due to some reason and once the problem is fixed, can we re-apply
        the file? That depends on whether the account data has been updated,
        e.g., whether there is a rollback mechanism in place when the process
        fails, or if the partially processed file has made its side-effect to
        the accounts, reapplying the fixed file may also fail again since some
        account might have been locked during the partial process. Allowing
        such transaction to go through as a duplicate makes it easier on this
        scenario. Please see the test cases in the appendix.
1. Input:
    1. Since the sample provided in the question sheet contains only 2 out of 5
    transaction types: deposit and withdrawal, I assume the literal of the
    other 3 in the input are: "dispute", "resolve" and "chargeback", all in
    lower case.
    1. The input is always a valid CSV - always have 4 fields like:
       ```
        type,    client, tx, amount
        deposit,      1,  1,    1.0
        deposit,      2,  2,    2.0
        dispute,      1,  1,
        ```
        and when the transaction does not require an amount, like "dispute", it
        will still come with a trailing comma `,`. I need to make such an
        assumption since this is not covered in the sample provided. The CSV
        parsing breaks otherwise.
    1. Amount for deposit and withdrawal transactions can only be non-negative
    numbers. Although in terms of algebraic operations, negative number will
    still work, I just want to call this out that there is no validation
    against negative numbers in the input file.

# Answers to some of the questions in the sheet:
1. Can you stream values through memory as opposed to loading the entire data
set upfront?\
This is entirely up to the implementation of the `std::io::Read` provided to
the `TransactionStreamProcessor`.
1. What if your code was bundled in a server, and these CSVs came from
thousands of concurrent TCP streams?\
It will still work with following limit:
    - All CSV files to be processed in parallel do not have common clients.\
        The reason is that the process of transactions from a single client
        needs to be total ordered, while process of transactions from different
        clients can be done in any order


# Limits
1. Amount has a range of `[i64::MIN / 10_000, i64::MAX / 10_000]`\
    I chose to store the amount as an integer for simplicity. But in order to
    keep 4 digits after the decimal point, I had to reduce the range by
    `10^4`.
1. Serde: not fully using serde (mostly due to time limit)
    1. Due to the lack of strongly typed deserialisation, the parsing would
    only fail when deposit or withdrawal does not have an "amount" field. But
    it would not fail when dispute, resolve or chargeback has an "amount"
    field.


# Appendix
## Test Cases
### [Depositor](src/account/transactors/depositor.rs)
```rust
//    |------------------- input ------------------| |-------------------- output --------------------------------------------------|
//
//     original_account,                   tx_id,                        expected_account
//        avail, deposits,                   amount, expected_status     avail,  deposits
#[case(active(0, vec![]),                      0, 3, Ok(Transacted),     active(3, vec![(0, accepted_dep(3))])                      )]
#[case(active(3, vec![(0, accepted_dep(3))]),  0, 3, Ok(Duplicate),      active(3, vec![(0, accepted_dep(3))])                      )]
#[case(active(3, vec![(0, held_dep(3))]),      0, 3, Ok(Duplicate),      active(3, vec![(0, held_dep(3))])                          )]
#[case(active(3, vec![(0, resolved_dep(3))]),  0, 3, Ok(Duplicate),      active(3, vec![(0, resolved_dep(3))])                      )]
#[case(active(3, vec![(0, chrgd_bck_dep(3))]), 0, 3, Ok(Duplicate),      active(3, vec![(0, chrgd_bck_dep(3))])                     )]
#[case(active(3, vec![(0, accepted_dep(3))]),  2, 5, Ok(Transacted),     active(8, vec![(0, accepted_dep(3)), (2, accepted_dep(5))]))]
// locked cases
#[case(locked(3, vec![(0, accepted_dep(3))]),  0, 3, Ok(Duplicate),      locked(3, vec![(0, accepted_dep(3))])                      )]
#[case(locked(3, vec![(0, held_dep(3))]),      0, 3, Ok(Duplicate),      locked(3, vec![(0, held_dep(3))])                          )]
#[case(locked(3, vec![(0, resolved_dep(3))]),  0, 3, Ok(Duplicate),      locked(3, vec![(0, resolved_dep(3))])                      )]
#[case(locked(3, vec![(0, chrgd_bck_dep(3))]), 0, 3, Ok(Duplicate),      locked(3, vec![(0, chrgd_bck_dep(3))])                     )]
#[case(locked(3, vec![(0, accepted_dep(3))]),  1, 3, Err(AccountLocked), locked(3, vec![(0, accepted_dep(3))])                      )]
```
### [Withdrawer](src/account/transactors/withdrawer.rs)
```rust
//    |-------------------- input -----------------------------| |------------------------------- output ----------------------------------|
//                                            tx
//     original_account,                      id,                                expected_account
//        avail, existing withdrawals,            amount, expected_status           avail, existing withdrawals
#[case(active(7, vec![]),                      0,      8, Err(InsufficientFund), active(7, vec![])                                          )]
#[case(active(7, vec![]),                      0,      0, Ok(Transacted),        active(7, vec![(0, accepted_wdr(0))])                      )]
#[case(active(7, vec![]),                      0,      4, Ok(Transacted),        active(3, vec![(0, accepted_wdr(4))])                      )]
#[case(active(7, vec![]),                      0,      7, Ok(Transacted),        active(0, vec![(0, accepted_wdr(7))])                      )]
#[case(active(7, vec![(0, accepted_wdr(3))]),  0,      3, Ok(Duplicate),         active(7, vec![(0, accepted_wdr(3))])                      )]
#[case(active(7, vec![(0, accepted_wdr(3))]),  1,      5, Ok(Transacted),        active(2, vec![(0, accepted_wdr(3)), (1, accepted_wdr(5))]))]
// locked cases
#[case(locked(7, vec![(0, accepted_wdr(3))]),  0,      3, Ok(Duplicate),         locked(7, vec![(0, accepted_wdr(3))])                      )]
#[case(locked(7, vec![(0, accepted_wdr(3))]),  1,      3, Err(AccountLocked),    locked(7, vec![(0, accepted_wdr(3))])                      )]
```
### [Disputer](src/account/transactors/disputer/credit_disputer.rs)
```rust
// disputing credit transactions
//    |------------------ input ---------------------| |------------------------- output -----------------------------------|
//     original_account,                            tx                           expected_account
//        avail, held, deposits,                    id, expected_status,             avail, held, deposits
#[case(active(7,    0, vec![(0, accepted_dep(3))] ), 0, Ok(Transacted),          active( 4,    3, vec![(0, held_dep(3))]     ))]
#[case(active(7,    0, vec![(0, held_dep(3))]     ), 0, Ok(Duplicate),           active( 7,    0, vec![(0, held_dep(3))]     ))]
#[case(active(7,    0, vec![(0, resolved_dep(3))] ), 0, Ok(Duplicate),           active( 7,    0, vec![(0, resolved_dep(3))] ))]
#[case(active(7,    0, vec![(0, chrgd_bck_dep(3))]), 0, Ok(Duplicate),           active( 7,    0, vec![(0, chrgd_bck_dep(3))]))]
#[case(active(3,    0, vec![(0, accepted_dep(7))] ), 0, Ok(Transacted),          active(-4,    7, vec![(0, held_dep(7))]     ))]
#[case(active(3,    0, vec![(0, accepted_dep(7))] ), 1, Err(NoTransactionFound), active( 3,    0, vec![(0, accepted_dep(7))] ))]
// locked cases
#[case(locked(7,    0, vec![(0, accepted_dep(3))] ), 0, Err(AccountLocked),      locked( 7,    0, vec![(0, accepted_dep(3))] ))]
#[case(locked(7,    0, vec![(0, accepted_dep(3))] ), 1, Err(AccountLocked),      locked( 7,    0, vec![(0, accepted_dep(3))] ))]
#[case(locked(7,    0, vec![(0, held_dep(3))]     ), 0, Ok(Duplicate),           locked( 7,    0, vec![(0, held_dep(3))]     ))]
#[case(locked(7,    0, vec![(0, resolved_dep(3))] ), 0, Ok(Duplicate),           locked( 7,    0, vec![(0, resolved_dep(3))] ))]
#[case(locked(7,    0, vec![(0, chrgd_bck_dep(3))]), 0, Ok(Duplicate),           locked( 7,    0, vec![(0, chrgd_bck_dep(3))]))]
```
### [Resolver](src/account/transactors/resolver/credit_resolver.rs)
```rust
// disputing credit transactions
//    |------------------ input -----------------------| |----------------------------- output ------------------------------------|
//     original_account,                            tx                               expected_account
//        avail, held, deposits,                    id, expected_status,                 avail, held, deposits
#[case(active(7,    5, vec![(0, held_dep(3))]),      0, Ok(Transacted),              active(10,    2, vec![(0, resolved_dep(3))]) )]
#[case(active(7,    0, vec![(0, resolved_dep(3))]),  0, Ok(Duplicate),               active( 7,    0, vec![(0, resolved_dep(3))]) )]
#[case(active(7,    0, vec![(0, accepted_dep(3))]),  0, Err(NonDisputedTransaction), active( 7,    0, vec![(0, accepted_dep(3))]) )]
#[case(active(7,    0, vec![(0, chrgd_bck_dep(3))]), 0, Err(NonDisputedTransaction), active( 7,    0, vec![(0, chrgd_bck_dep(3))]))]
#[case(active(7,    0, vec![(0, chrgd_bck_dep(3))]), 1, Err(NoTransactionFound),     active( 7,    0, vec![(0, chrgd_bck_dep(3))]))]
// locked cases
#[case(locked(7,    5, vec![(0, held_dep(3))]),      0, Err(AccountLocked),          locked( 7,    5, vec![(0, held_dep(3))])     )]
#[case(locked(7,    0, vec![(0, resolved_dep(3))]),  0, Ok(Duplicate),               locked( 7,    0, vec![(0, resolved_dep(3))]) )]
#[case(locked(7,    0, vec![(0, accepted_dep(3))]),  0, Err(AccountLocked),          locked( 7,    0, vec![(0, accepted_dep(3))]) )]
#[case(locked(7,    0, vec![(0, chrgd_bck_dep(3))]), 0, Err(AccountLocked),          locked( 7,    0, vec![(0, chrgd_bck_dep(3))]))]
#[case(locked(7,    0, vec![(0, chrgd_bck_dep(3))]), 1, Err(AccountLocked),          locked( 7,    0, vec![(0, chrgd_bck_dep(3))]))]
```
### [Backcharger](src/account/transactors/backcharger/credit_backcharger.rs)
```rust
// disputing credit transactions
//    |------------------ input ----------------------| |---------------------------- output ------------------------------------|
//     original_account,                            tx                               expected_account
//        avail, held, deposits,                    id, expected_status,                avail, held, deposits
#[case(active(7,    5, vec![(0, accepted_dep(3))]),  0, Err(NonDisputedTransaction), active(7,    5, vec![(0, accepted_dep(3))]) )]
#[case(active(7,    5, vec![(0, held_dep(3))]),      0, Ok(Transacted),              locked(7,    2, vec![(0, chrgd_bck_dep(3))]))]
#[case(active(7,    5, vec![(0, resolved_dep(3))]),  0, Err(NonDisputedTransaction), active(7,    5, vec![(0, resolved_dep(3))]) )]
#[case(active(7,    5, vec![(0, chrgd_bck_dep(3))]), 0, Ok(Duplicate),               active(7,    5, vec![(0, chrgd_bck_dep(3))]))]
#[case(active(7,    5, vec![(0, chrgd_bck_dep(3))]), 1, Err(NoTransactionFound),     active(7,    5, vec![(0, chrgd_bck_dep(3))]))]
// locked cases
#[case(locked(7,    5, vec![(0, accepted_dep(3))]),  0, Err(AccountLocked),          locked(7,    5, vec![(0, accepted_dep(3))]) )]
#[case(locked(7,    5, vec![(0, held_dep(3))]),      0, Err(AccountLocked),          locked(7,    5, vec![(0, held_dep(3))])     )]
#[case(locked(7,    5, vec![(0, resolved_dep(3))]),  0, Err(AccountLocked),          locked(7,    5, vec![(0, resolved_dep(3))]) )]
#[case(locked(7,    5, vec![(0, chrgd_bck_dep(3))]), 0, Ok(Duplicate),               locked(7,    5, vec![(0, chrgd_bck_dep(3))]))]
#[case(locked(7,    5, vec![(0, chrgd_bck_dep(3))]), 1, Err(AccountLocked),          locked(7,    5, vec![(0, chrgd_bck_dep(3))]))]
```
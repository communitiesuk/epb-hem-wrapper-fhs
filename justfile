unit:
    cargo test --lib

e2e:
    cargo test --test test_future_homes_standard_preprocessing -- --skip test_preprocessed_input_matches_expected

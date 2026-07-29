unit:
    cargo test --lib

e2e:
    cargo test --test test_future_homes_standard_preprocessing -- --skip test_preprocessed_input_matches_expected

e2e-actual:
    cargo test --test test_future_homes_standard_actual

generate-python-outputs:
    cargo run --bin generate_python_outputs

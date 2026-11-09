unit:
    cargo test --lib

e2e:
    cargo test --release --no-fail-fast \
        --test test_future_homes_standard_preprocessing \
        --test test_future_homes_standard_postproc \
        --test test_future_homes_standard_actual \
        -- --nocapture

e2e-preproc:
    cargo test --test test_future_homes_standard_preprocessing --release -- --nocapture

e2e-postproc:
    cargo test --test test_future_homes_standard_postproc --release -- --nocapture

e2e-actual:
    cargo test --test test_future_homes_standard_actual --release -- --nocapture

generate-python-outputs:
    cargo run --bin generate_python_outputs

e2e-preproc-provided:
    cargo test test_fhs_preprocessing_output_against_provided_results

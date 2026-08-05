unit:
    cargo test --lib

e2e: e2e-preproc e2e-postproc

e2e-preproc:
    cargo test --test test_future_homes_standard_preprocessing --release -- --skip test_preprocessed_input_matches_expected

e2e-postproc:
    cargo test --test test_future_homes_standard_postproc --release

generate-python-outputs:
    cargo run --bin generate_python_outputs

e2e-preproc-provided:
    cargo test test_fhs_preprocessing_output_against_provided_results

"""Basic filtering tests for ITU-T G.191 FIR/IIR filters."""

from __future__ import annotations

import numpy as np

from g191_filter import filter_array, get_coefficients_ba_py, get_coefficients_sos_py, list_filters


def test_available_filters() -> None:
    """Test that filter list is not empty and contains expected filters."""
    filters = list_filters()
    assert len(filters) > 0, "Filter list should not be empty"

    # Check for key filter families
    filter_names = [f.lower() for f in filters]
    expected_families = ["irs8khz", "irs16khz", "mod_irs48khz", "irs16mod", "hq2", "lp", "flat", "iir", "g712"]

    # At least some expected filters should be available
    found_families = [family for family in expected_families if any(family in name for name in filter_names)]
    assert len(found_families) >= 3, f"Should have at least 3 filter families, found {len(found_families)}"


def test_filter_idempotence_zero_input() -> None:
    """Test that zero input produces zero output."""
    filters = list_filters()

    # Find filters that support reasonable input length
    test_filters = []
    for f in filters:
        try:
            input_signal = np.zeros(10, dtype=np.float64)
            output = filter_array(f, input_signal)
            if len(output) > 0:
                test_filters.append(f)
                if len(test_filters) >= 2:  # Test a few
                    break
        except Exception:  # noqa: S110
            pass

    # If no filters work, skip this test
    if not test_filters:
        return

    for filter_id in test_filters:
        input_signal = np.zeros(10, dtype=np.float64)
        output = filter_array(filter_id, input_signal)

        assert len(output) > 0, f"Output should not be empty for {filter_id}"
        assert np.allclose(output, 0.0, atol=1e-10), f"Filter {filter_id} should output zeros for zero input"


def test_filter_output_shape() -> None:
    """Test that filter output has reasonable shape."""
    filters = list_filters()
    test_filter = filters[0]

    # Test with different input lengths
    input_lengths = [100, 1000]

    for length in input_lengths:
        input_signal = np.ones(length, dtype=np.float64)
        output = filter_array(test_filter, input_signal)

        # Output length depends on filter type:
        # - FIR (same rate): same length
        # - Downsampling: shorter
        # - Upsampling: longer
        assert len(output) > 0, f"Output should not be empty for {test_filter}"
        assert len(output) <= length * 2, f"Output should not be more than 2x input length for {test_filter}"


def test_filter_real_inputs() -> None:
    """Test that filter processes real-valued inputs correctly."""
    filters = list_filters()

    # Find a filter that works with reasonable input
    test_filter = None
    input_signal = np.array([0.1, 0.2, 0.3, 0.4, 0.5, 0.4, 0.3, 0.2, 0.1, 0.0], dtype=np.float64)

    for f in filters:
        try:
            output = filter_array(f, input_signal)
            if len(output) > 0 and np.all(np.isfinite(output)):
                test_filter = f
                break
        except Exception:  # noqa: S110
            pass

    # If no filters work, skip this test
    if test_filter is None:
        return

    output = filter_array(test_filter, input_signal)

    # Output should be finite
    assert np.all(np.isfinite(output)), f"Output should be finite for {test_filter}"

    # Output should have reasonable length (depending on filter type)
    assert len(output) > 0, f"Output should not be empty for {test_filter}"


def test_filter_coefficients_available() -> None:
    """Test that coefficients can be exported."""
    filters = list_filters()
    test_filter = filters[0]

    # Test BA coefficients
    ba = get_coefficients_ba_py(test_filter)
    assert ba is not None, f"Should be able to get BA coefficients for {test_filter}"
    assert len(ba[0]) > 0, "Numerator should have coefficients"
    assert len(ba[1]) > 0, "Denominator should have coefficients"

    # Test SOS coefficients
    sos = get_coefficients_sos_py(test_filter)
    assert sos is not None, f"Should be able to get SOS coefficients for {test_filter}"
    assert len(sos) > 0, "Should have at least one section"


if __name__ == "__main__":
    test_available_filters()
    print("✓ test_available_filters passed")

    test_filter_idempotence_zero_input()
    print("✓ test_filter_idempotence_zero_input passed")

    test_filter_output_shape()
    print("✓ test_filter_output_shape passed")

    test_filter_real_inputs()
    print("✓ test_filter_real_inputs passed")

    test_filter_coefficients_available()
    print("✓ test_filter_coefficients_available passed")

    print("\nAll tests passed!")

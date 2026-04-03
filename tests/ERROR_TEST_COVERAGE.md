# Error Test Coverage for m6502.asm

This document tracks which error conditions from m6502.asm are covered by tests.

## Tested Error Conditions ✅

The following errors are covered by automated tests in `tests/basic_programs/`:

1. **ERRNF** - "NEXT WITHOUT FOR"
   - Test: `error_next_without_for.bas`
   - Triggers: NEXT statement without matching FOR loop

2. **ERRSN** - "SYNTAX"  
   - Test: `error_syntax.bas`
   - Triggers: Invalid expression syntax (e.g., `5 + * 3`)

3. **ERRRG** - "RETURN WITHOUT GOSUB"
   - Test: `error_return_without_gosub.bas`
   - Triggers: RETURN statement without matching GOSUB

4. **ERROD** - "OUT OF DATA"
   - Test: `error_out_of_data.bas`
   - Triggers: READ statement when all DATA is exhausted

5. **ERRDV0** - "DIVISION BY ZERO" (/0)
   - Test: `error_division_by_zero.bas`
   - Triggers: Division or modulo by zero

6. **ERRUS** - "UNDEF'D STATEMENT"
   - Test: `error_undef_statement.bas`
   - Triggers: GOTO/GOSUB to nonexistent line number

7. **ERRBS** - "BAD SUBSCRIPT"
   - Test: `array_bounds_error.bas`
   - Triggers: Array index out of bounds

8. **ERRDD** - "REDIM'D ARRAY"
   - Test: `error_redim_array.bas`
   - Triggers: Attempting to DIM an array twice

9. **ERRTM** - "TYPE MISMATCH"
   - Test: `error_type_mismatch.bas`
   - Triggers: Using string where number expected or vice versa

10. **ERRUF** - "UNDEF'D FUNCTION"
    - Test: `error_undef_function.bas`
    - Triggers: Using FN function that wasn't DEF'd

11. **ERRFC** - "ILLEGAL QUANTITY"
    - Test: `error_illegal_quantity.bas`
    - Triggers: Invalid argument to function (e.g., SQR(-1))

12. **ERROV** - "OVERFLOW"
    - Test: `error_overflow.bas`
    - Triggers: Arithmetic result too large

## Untested Error Conditions ⚠️

The following errors exist in m6502.asm but are difficult to trigger with simple test programs:

1. **ERRLS** - "STRING TOO LONG"
   - Requires string concatenation result >= 256 characters
   - Test file exists but is commented out in test suite
   - The interpreter may handle this gracefully or have different limits

2. **ERRST** - "FORMULA TOO COMPLEX"
   - Requires exhausting all string temporary storage
   - Test file exists but is commented out in test suite
   - Very difficult to trigger in practice

3. **ERROM** - "OUT OF MEMORY"
   - Would require allocating memory until exhaustion
   - No test created (would be system-dependent)

4. **ERRID** - "ILLEGAL DIRECT"
   - Commands that can't be used in immediate mode
   - No test created (currently all commands accepted)

5. **ERRCN** - "CAN'T CONTINUE"
   - Resume/CONT without a stopped program  
   - No test created (CONT not yet implemented)

6. **ERRBD** - "FILE DATA"
   - File I/O error (conditional on EXTIO feature)
   - No test created (file I/O not implemented)

## Test Execution

Run all error tests:
```bash
cargo test --release test_error
```

Run all tests including interpreter tests:
```bash
cargo test --release
```

## Summary

- **12 of 18** error conditions have automated test coverage
- **2** have test files but are commented out (difficult to trigger)
- **4** are not tested (require features not yet implemented or are system-dependent)

The core error handling paths are well-covered by the test suite.

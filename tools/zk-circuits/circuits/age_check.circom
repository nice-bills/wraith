pragma circom 2.2.3;

include "circomlib/circuits/comparators.circom";

template AgeVerifier() {
    // Public inputs
    signal input minAge;
    signal input allowedNationality1;
    signal input allowedNationality2;
    signal input allowedNationality3;

    // Private inputs (from passport)
    signal input age;
    signal input nationality;
    signal input passportValid;

    signal output verified;

    // 1. Age check: age >= minAge using proper GreaterEqThan
    component ageGte = GreaterEqThan(32);
    ageGte.in[0] <== age;
    ageGte.in[1] <== minAge;

    // 2. Nationality check: nationality in {nat1, nat2, nat3}
    component natEq1 = IsEqual();
    natEq1.in[0] <== nationality;
    natEq1.in[1] <== allowedNationality1;

    component natEq2 = IsEqual();
    natEq2.in[0] <== nationality;
    natEq2.in[1] <== allowedNationality2;

    component natEq3 = IsEqual();
    natEq3.in[0] <== nationality;
    natEq3.in[1] <== allowedNationality3;

    // Sum of matches (each is 0 or 1)
    // nationalityAllowed = 1 if nationality matches one of the allowed
    signal nationalityAllowed;
    nationalityAllowed <== natEq1.out + natEq2.out + natEq3.out;

    // 3. Passport must be valid - constrain to boolean (0 or 1)
    // This is a quadratic constraint
    passportValid * (passportValid - 1) === 0;

    // 4. All must pass - use explicit constraints for each step
    // ageNatValid = ageGte.out AND nationalityAllowed
    // Since ageGte.out is 0 or 1 and nationalityAllowed is 0-3,
    // we need to ensure nationalityAllowed >= 1 means a match
    signal ageNatValid;
    ageNatValid <== ageGte.out * nationalityAllowed;

    // verified = ageNatValid AND passportValid
    signal validPass;
    validPass <== ageNatValid * passportValid;

    verified <== validPass;
}

component main {public [minAge, allowedNationality1, allowedNationality2, allowedNationality3]} = AgeVerifier();
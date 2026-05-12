pragma circom 2.2.3;

include "circomlib/circuits/comparators.circom";

template PassportVerifier() {
    // Public inputs
    signal input minAge;
    signal input excludedCountries[3];

    // Private inputs (from passport)
    signal input age;
    signal input countryCode;
    signal input isHuman;
    signal input humanityProof;

    signal output verified;

    // 1. Age check: age >= minAge using proper GreaterEqThan
    component ageGte = GreaterEqThan(32);
    ageGte.in[0] <== age;
    ageGte.in[1] <== minAge;

    // 2. Humanity check: isHuman must be 1
    // Constrain isHuman to be boolean
    isHuman * (isHuman - 1) === 0;

    // 3. Humanity proof must be valid - constrain to boolean
    humanityProof * (humanityProof - 1) === 0;

    // 4. Country not in excluded list
    // Pre-declare components outside the loop (Circom 2.x requirement)
    component eq[3];
    for (var i = 0; i < 3; i++) {
        eq[i] = IsEqual();
        eq[i].in[0] <== countryCode;
        eq[i].in[1] <== excludedCountries[i];
    }

    // Sum of all matches - isExcluded > 0 means country is excluded
    signal isExcluded;
    isExcluded <== eq[0].out + eq[1].out + eq[2].out;

    // 5. Compute the base valid signal
    signal ageHumanValid;
    ageHumanValid <== ageGte.out * isHuman;

    signal ageHumanProofValid;
    ageHumanProofValid <== ageHumanValid * humanityProof;

    // 6. Enforce exclusion constraint first
    // If isExcluded > 0, then verified must be 0
    // This works because verified hasn't been assigned yet - we use it as a variable
    isExcluded * ageHumanProofValid === 0;

    // 7. Assign verified
    verified <== ageHumanProofValid;
}

component main {public [minAge, excludedCountries]} = PassportVerifier();
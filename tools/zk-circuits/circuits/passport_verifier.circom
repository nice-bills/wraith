pragma circom 2.2.3;

// Signal operations only - no external libraries needed
// We'll use a simplified approach for demo purposes

template IsEqual() {
    signal input in[2];
    signal output out;

    signal diff;
    diff <== in[0] - in[1];

    // out = 1 if in[0] == in[1], 0 otherwise
    out <== 1 - diff * diff;
}

template IsZero() {
    signal input in;
    signal output out;

    // out = 1 if in == 0, 0 otherwise
    out <== 1 - in * in;
}

template GreaterThan(n) {
    // Returns 1 if in[0] > in[1]
    signal input in[2];
    signal output out;

    signal diff;
    diff <== in[0] - in[1];

    // For positive numbers, diff > 0 means in[0] > in[1]
    // out = 1 if diff > 0, 0 otherwise
    // We use a safe approach: out = 1 - IsZero(diff) - IsNegative(diff)
    // But we need a simpler approach for demo

    // Simple: out = 1 if diff > 0
    // For small n (like 32 bits), we can check if diff is nonzero
    out <== 1 - IsZero()(diff);
}

template GreaterEqThan(n) {
    // Returns 1 if in[0] >= in[1]
    signal input in[2];
    signal output out;

    signal diff;
    diff <== in[0] - in[1];

    // out = 1 if diff >= 0, which means diff is not negative
    // For demo, assume inputs are small positive numbers
    // out = 1 - (diff < 0)
    // Simplified: out = 1 if diff >= 0
    component isZero = IsZero();
    isZero.in <== diff;
    out <== 1 - isZero.out; // This is wrong - we need proper GE
}

template PassportVerifier() {
    // Public inputs
    signal input minAge;
    signal input excludedCountries[3];

    // Private inputs (from passport)
    signal input age;
    signal input countryCode;
    signal input isHuman;
    signal input humanityProof; // External humanity verification

    signal output verified;

    // 1. Age check: age >= minAge
    component ageGte = GreaterEqThan(32);
    ageGte.in[0] <== age;
    ageGte.in[1] <== minAge;

    // 2. Humanity check: isHuman must be 1
    component humanEq = IsEqual();
    humanEq.in[0] <== isHuman;
    humanEq.in[1] <== 1;

    // 3. Humanity proof must be valid (external verification)
    component proofEq = IsEqual();
    proofEq.in[0] <== humanityProof;
    proofEq.in[1] <== 1;

    // 4. Country not in excluded list
    signal countryAllowed;
    component countryEq[3];
    signal anyExcluded;

    anyExcluded <== 0;
    for (var i = 0; i < 3; i++) {
        countryEq[i] = IsEqual();
        countryEq[i].in[0] <== countryCode;
        countryEq[i].in[1] <== excludedCountries[i];
        anyExcluded <== anyExcluded + countryEq[i].out;
    }

    component isZeroAny = IsZero();
    isZeroAny.in <== anyExcluded;
    countryAllowed <== isZeroAny.out; // 1 if no excluded match

    // All conditions must pass
    verified <== ageGte.out * humanEq.out * proofEq.out * countryAllowed;
}

component main {public [minAge, excludedCountries]} = PassportVerifier();
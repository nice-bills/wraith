pragma circom 2.2.3;

// Simple age verification circuit
// Proves: age >= 18 AND nationality is allowed AND passport is valid

template IsZero() {
    signal input in;
    signal output out;
    out <== 1 - in * in;
}

template IsEqual() {
    signal input in[2];
    signal output out;
    signal diff;
    diff <== in[0] - in[1];
    out <== 1 - diff * diff;
}

template GreaterThan() {
    // Returns 1 if a > b
    signal input a;
    signal input b;
    signal output out;

    // a > b means a - b >= 1
    signal diff;
    diff <== a - b;

    // diff * (diff - 1) = 0 means diff is 0 or 1
    // We want: out = 1 if diff >= 1, 0 if diff <= 0
    // Using: out = diff * (1 - IsZero()(diff))
    component isZero = IsZero();
    isZero.in <== diff;

    // out = diff * (1 - isZero.out) = diff if diff != 0
    // This works for diff >= 0
    out <== diff * (1 - isZero.out);
}

template GreaterEqThan() {
    signal input a;
    signal input b;
    signal output out;

    // a >= b means a > b OR a == b
    component gt = GreaterThan();
    gt.a <== a;
    gt.b <== b;

    component eq = IsEqual();
    eq.in[0] <== a;
    eq.in[1] <== b;

    out <== gt.out + eq.out;
}

template AgeVerifier() {
    // Public
    signal input minAge;
    signal input allowedNationality1;
    signal input allowedNationality2;
    signal input allowedNationality3;

    // Private (from passport)
    signal input age;
    signal input nationality;
    signal input passportValid;

    signal output verified;

    // 1. Age check: age >= minAge
    component ageGte = GreaterEqThan();
    ageGte.a <== age;
    ageGte.b <== minAge;

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

    signal nationalityAllowed;
    nationalityAllowed <== natEq1.out + natEq2.out + natEq3.out;

    // 3. Passport must be valid
    component validEq = IsEqual();
    validEq.in[0] <== passportValid;
    validEq.in[1] <== 1;

    // All must pass (quadratic constraint: pairwise multiplications)
    signal tmp;
    tmp <== ageGte.out * nationalityAllowed;
    verified <== tmp * validEq.out;
}

component main {public [minAge, allowedNationality1, allowedNationality2, allowedNationality3]} = AgeVerifier();
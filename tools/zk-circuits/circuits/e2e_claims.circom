pragma circom 2.2.3;

// E2E-only: three public signals map to contract claim indices 0/1/2.
template E2eClaims() {
    signal input age;
    signal input country_code;
    signal input is_human;
}

component main {public [age, country_code, is_human]} = E2eClaims();

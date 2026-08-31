#!/bin/bash
sed -i '/<<<<<<< HEAD/,/=======/{
  /<<<<<<< HEAD/d
  /=======/d
}
/>>>>>>> origin\/main/d' soroban_cost_lints/src/lib.rs

sed -i '/<<<<<<< HEAD/,/=======/{
  /<<<<<<< HEAD/d
  /=======/d
}
/>>>>>>> origin\/main/d' docs/false_positives.md

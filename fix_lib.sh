#!/bin/bash
sed -i '/<<<<<<< HEAD/,/=======/{
  /<<<<<<< HEAD/d
  /=======/d
}
/>>>>>>> main/d' soroban_cost_lints/src/lib.rs

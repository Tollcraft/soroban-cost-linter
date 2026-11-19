# Table of contents

* [Introduction](README.md)

## Concepts

* [Cost Rationale](cost_rationale.md)
* [Lint Categories](lint_categories.md)

## Guides

* [Scope: Clippy vs. soroban-cost-linter](scope_boundary.md)
* [Integration Guide](integration.md)
* [Windows Setup Guide](windows_setup.md)
* [Handling False Positives](false_positives.md)
* [Severity Levels & Rationale](severity_rationale.md)
* [Troubleshooting](troubleshooting.md)

## Lints

* [Lint Catalog](lint_catalog.md)
* [Lint Reference](lints/README.md)
  * [host\_in\_loop](lints/host_in_loop.md)
  * [string\_concat\_in\_loop](lints/string_concat_in_loop.md)
  * [crypto\_hash\_of\_constant](lints/crypto_hash_of_constant.md)
  * [soroban\_storage\_in\_loop](lints/soroban_storage_in_loop.md)
  * [redundant\_env\_clone](lints/redundant_env_clone.md)
  * [unnecessary\_host\_function\_call](lints/unnecessary_host_function_call.md)
  * [soroban\_redundant\_storage\_read](lints/soroban_redundant_storage_read.md)
  * [storage\_read\_modify\_write](lints/storage_read_modify_write.md)
  * [storage\_write\_without\_read](lints/storage_write_without_read.md)
  * [discarded\_storage\_read](lints/discarded_storage_read.md)
  * [instance\_storage\_for\_unbounded\_data](lints/instance_storage_for_unbounded_data.md)
  * [persistent\_read\_without\_ttl\_extension](lints/persistent_read_without_ttl_extension.md)
  * [loop\_invariant\_storage\_access](lints/loop_invariant_storage_access.md)
  * [storage\_key\_construction\_in\_loop](lints/storage_key_construction_in_loop.md)
  * [bytes\_append\_in\_loop](lints/bytes_append_in_loop.md)
  * [unbounded\_input\_loop](lints/unbounded_input_loop.md)
  * [unnecessary\_string\_to\_bytes](lints/unnecessary_string_to_bytes.md)
  * [map\_insert\_in\_loop](lints/map_insert_in_loop.md)
  * [inefficient\_bytes\_concat](lints/inefficient_bytes_concat.md)
  * [contract\_call\_in\_loop](lints/contract_call_in_loop.md)
  * [extend\_ttl\_in\_loop](lints/extend_ttl_in_loop.md)
  * [formatted\_panic\_payload](lints/formatted_panic_payload.md)
  * [linear\_scan\_in\_loop](lints/linear_scan_in_loop.md)
  * [require\_auth\_in\_loop](lints/require_auth_in_loop.md)
  * [signature\_verification\_in\_loop](lints/signature_verification_in_loop.md)
  * [symbol\_new\_for\_short\_literal](lints/symbol_new_for_short_literal.md)
  * [unbounded\_recursion](lints/unbounded_recursion.md)
  * [unwrap\_on\_storage\_get](lints/unwrap_on_storage_get.md)
  * [vec\_where\_slice\_could\_be\_used](lints/vec_where_slice_could_be_used.md)
  * [val\_conversion\_chain](lints/val_conversion_chain.md)
  * [soroban\_inefficient\_bytes\_concat](lints/soroban_inefficient_bytes_concat.md)
  * [duplicate\_storage\_key\_construction](lints/duplicate_storage_key_construction.md)
  * [option\_wrapping\_in\_storage](lints/option_wrapping_in_storage.md)
  * [ledger\_context\_read\_in\_loop](lints/ledger_context_read_in_loop.md)
  * [redundant\_require\_auth](lints/redundant_require_auth.md)
  * [large\_constant\_array](lints/large_constant_array.md)
  * [collection\_len\_in\_loop\_condition](lints/collection_len_in_loop_condition.md)
  * [cross\_contract\_result\_discarded](lints/cross_contract_result_discarded.md)
  * [redundant\_val\_conversion](lints/redundant_val_conversion.md)
  * [instance\_storage\_write\_in\_loop](lints/instance_storage_write_in_loop.md)
  * [nested\_loop\_storage\_access](lints/nested_loop_storage_access.md)
  * [persistent\_storage\_for\_ephemeral\_data](lints/persistent_storage_for_ephemeral_data.md)
  * [redundant\_address\_clone](lints/redundant_address_clone.md)
  * [blind\_storage\_write](lints/blind_storage_write.md)
  * [bytes\_slice\_copy\_in\_loop](lints/bytes_slice_copy_in_loop.md)
  * [excessive\_vec\_capacity](lints/excessive_vec_capacity.md)

## Branding

* [Brand & Design Guide](branding.md)

## History

* [Phase 1 Analysis (Task 27)](history/PHASE_1_ANALYSIS_TASK_27.md)
* [Phase 1 Reconnaissance Analysis](history/PHASE_1_RECONNAISSANCE_ANALYSIS.md)
* [Roadmap MVP](history/roadmap_mvp.md)


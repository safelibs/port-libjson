#include "config.h"

#include <stddef.h>
#include <stdio.h>

#include "arraylist.h"
#include "json_object_iterator.h"
#include "json_patch.h"
#include "json_tokener.h"
#include "linkhash.h"
#include "printbuf.h"

_Static_assert(sizeof(struct array_list) == 32, "array_list size mismatch");
_Static_assert(offsetof(struct array_list, array) == 0, "array_list.array offset mismatch");
_Static_assert(offsetof(struct array_list, length) == 8, "array_list.length offset mismatch");
_Static_assert(offsetof(struct array_list, size) == 16, "array_list.size offset mismatch");
_Static_assert(offsetof(struct array_list, free_fn) == 24, "array_list.free_fn offset mismatch");

_Static_assert(sizeof(struct lh_entry) == 40, "lh_entry size mismatch");
_Static_assert(offsetof(struct lh_entry, k) == 0, "lh_entry.k offset mismatch");
_Static_assert(offsetof(struct lh_entry, k_is_constant) == 8, "lh_entry.k_is_constant offset mismatch");
_Static_assert(offsetof(struct lh_entry, v) == 16, "lh_entry.v offset mismatch");
_Static_assert(offsetof(struct lh_entry, next) == 24, "lh_entry.next offset mismatch");
_Static_assert(offsetof(struct lh_entry, prev) == 32, "lh_entry.prev offset mismatch");

_Static_assert(sizeof(struct lh_table) == 56, "lh_table size mismatch");
_Static_assert(offsetof(struct lh_table, size) == 0, "lh_table.size offset mismatch");
_Static_assert(offsetof(struct lh_table, count) == 4, "lh_table.count offset mismatch");
_Static_assert(offsetof(struct lh_table, head) == 8, "lh_table.head offset mismatch");
_Static_assert(offsetof(struct lh_table, tail) == 16, "lh_table.tail offset mismatch");
_Static_assert(offsetof(struct lh_table, table) == 24, "lh_table.table offset mismatch");
_Static_assert(offsetof(struct lh_table, free_fn) == 32, "lh_table.free_fn offset mismatch");
_Static_assert(offsetof(struct lh_table, hash_fn) == 40, "lh_table.hash_fn offset mismatch");
_Static_assert(offsetof(struct lh_table, equal_fn) == 48, "lh_table.equal_fn offset mismatch");

_Static_assert(sizeof(struct printbuf) == 16, "printbuf size mismatch");
_Static_assert(offsetof(struct printbuf, buf) == 0, "printbuf.buf offset mismatch");
_Static_assert(offsetof(struct printbuf, bpos) == 8, "printbuf.bpos offset mismatch");
_Static_assert(offsetof(struct printbuf, size) == 12, "printbuf.size offset mismatch");

_Static_assert(sizeof(struct json_object_iter) == 24, "json_object_iter size mismatch");
_Static_assert(offsetof(struct json_object_iter, key) == 0, "json_object_iter.key offset mismatch");
_Static_assert(offsetof(struct json_object_iter, val) == 8, "json_object_iter.val offset mismatch");
_Static_assert(offsetof(struct json_object_iter, entry) == 16, "json_object_iter.entry offset mismatch");

_Static_assert(sizeof(struct json_object_iterator) == 8, "json_object_iterator size mismatch");
_Static_assert(offsetof(struct json_object_iterator, opaque_) == 0,
               "json_object_iterator.opaque_ offset mismatch");

_Static_assert(sizeof(struct json_tokener_srec) == 32, "json_tokener_srec size mismatch");
_Static_assert(offsetof(struct json_tokener_srec, obj) == 8, "json_tokener_srec.obj offset mismatch");
_Static_assert(offsetof(struct json_tokener_srec, current) == 16,
               "json_tokener_srec.current offset mismatch");
_Static_assert(offsetof(struct json_tokener_srec, obj_field_name) == 24,
               "json_tokener_srec.obj_field_name offset mismatch");

_Static_assert(sizeof(struct json_tokener) == 72, "json_tokener size mismatch");
_Static_assert(offsetof(struct json_tokener, char_offset) == 32,
               "json_tokener.char_offset offset mismatch");
_Static_assert(offsetof(struct json_tokener, quote_char) == 48,
               "json_tokener.quote_char offset mismatch");
_Static_assert(offsetof(struct json_tokener, stack) == 56, "json_tokener.stack offset mismatch");
_Static_assert(offsetof(struct json_tokener, flags) == 64, "json_tokener.flags offset mismatch");

_Static_assert(sizeof(struct json_patch_error) == 24, "json_patch_error size mismatch");
_Static_assert(offsetof(struct json_patch_error, errno_code) == 0,
               "json_patch_error.errno_code offset mismatch");
_Static_assert(offsetof(struct json_patch_error, patch_failure_idx) == 8,
               "json_patch_error.patch_failure_idx offset mismatch");
_Static_assert(offsetof(struct json_patch_error, errmsg) == 16,
               "json_patch_error.errmsg offset mismatch");

int main(void)
{
    puts("abi_layout_ok");
    return 0;
}

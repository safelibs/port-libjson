#ifdef NDEBUG
#undef NDEBUG
#endif

#include <assert.h>
#include <errno.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "arraylist.h"
#include "debug.h"
#include "json_c_version.h"
#include "json_util.h"
#include "linkhash.h"
#include "printbuf.h"

extern char *_json_c_strerror(int errno_in);
extern void _json_c_set_last_err(const char *err_fmt, ...);
extern const char *json_hex_chars;
extern const char *json_number_chars;
extern int json_c_get_random_seed(void);

static int free_count = 0;

static char *dup_cstr(const char *input)
{
	size_t len = strlen(input) + 1;
	char *copy = malloc(len);

	assert(copy != NULL);
	memcpy(copy, input, len);
	return copy;
}

static void free_int(void *data)
{
	if (data != NULL)
	{
		free_count++;
		free(data);
	}
}

static void free_entry(struct lh_entry *entry)
{
	if (!entry->k_is_constant)
		free((void *)entry->k);
	free((void *)entry->v);
	free_count++;
}

static unsigned long perllike_hash(const char *text)
{
	unsigned long hashval = 1;

	while (*text != '\0')
		hashval = hashval * 33 + *text++;
	return hashval;
}

static int *new_int(int value)
{
	int *slot = malloc(sizeof(*slot));

	assert(slot != NULL);
	*slot = value;
	return slot;
}

static void test_array_list(void)
{
	struct array_list *list;

	free_count = 0;
	list = array_list_new2(free_int, 2);
	assert(list != NULL);
	assert(list->array != NULL);
	assert(list->length == 0);
	assert(list->size == 2);

	assert(array_list_add(list, new_int(1)) == 0);
	assert(array_list_put_idx(list, 3, new_int(4)) == 0);
	assert(list->length == 4);
	assert(list->array[1] == NULL);
	assert(list->array[2] == NULL);

	assert(array_list_insert_idx(list, 1, new_int(2)) == 0);
	assert(list->length == 5);
	assert(*(int *)array_list_get_idx(list, 0) == 1);
	assert(*(int *)array_list_get_idx(list, 1) == 2);
	assert(*(int *)array_list_get_idx(list, 4) == 4);

	assert(array_list_del_idx(list, 1, 3) == 0);
	assert(list->length == 2);
	assert(free_count == 1);
	assert(*(int *)array_list_get_idx(list, 1) == 4);

	assert(array_list_shrink(list, 1) == 0);
	assert(list->size == 3);

	array_list_free(list);
	assert(free_count == 3);
}

static void test_printbuf(void)
{
	struct printbuf *pb = printbuf_new();

	assert(pb != NULL);
	assert(pb->buf != NULL);
	assert(pb->size == 32);
	assert(pb->bpos == 0);
	assert(pb->buf[0] == '\0');

	assert(printbuf_memappend(pb, "abc", 3) == 3);
	assert(pb->bpos == 3);
	assert(strcmp(pb->buf, "abc") == 0);

	assert(printbuf_memset(pb, -1, 'x', 2) == 0);
	assert(pb->bpos == 5);
	assert(memcmp(pb->buf, "abcxx", 5) == 0);

	errno = 0;
	assert(printbuf_memappend(pb, "z", -1) == -1);
	assert(errno == EFBIG);

	printbuf_reset(pb);
	assert(pb->bpos == 0);
	assert(pb->buf[0] == '\0');

	assert(sprintbuf(pb, "%s:%d", "id", 9) == 4);
	assert(strcmp(pb->buf, "id:9") == 0);

	printbuf_free(pb);
}

static void test_hash_modes(void)
{
	struct lh_table *table;
	const char high_bytes[] = { (char)0xff, '\0' };

	assert(json_global_set_string_hash(JSON_C_STR_HASH_PERLLIKE) == 0);
	table = lh_kchar_table_new(4, NULL);
	assert(table != NULL);
	assert(table->hash_fn != NULL);
	assert(table->hash_fn("abc") == perllike_hash("abc"));
	assert(table->hash_fn(high_bytes) == perllike_hash(high_bytes));
	lh_table_free(table);

	assert(json_global_set_string_hash(JSON_C_STR_HASH_DFLT) == 0);
	table = lh_kchar_table_new(4, NULL);
	assert(table != NULL);
	assert(table->hash_fn != NULL);
	(void)table->hash_fn("abc");
	lh_table_free(table);

	assert(json_global_set_string_hash(99) == -1);
}

static void test_linkhash(void)
{
	struct lh_entry *gamma;
	struct lh_table *table;
	void *found = NULL;

	free_count = 0;
	table = lh_kchar_table_new(2, free_entry);
	assert(table != NULL);
	assert(table->count == 0);
	assert(table->head == NULL);

	assert(lh_table_insert(table, dup_cstr("alpha"), new_int(1)) == 0);
	assert(lh_table_insert_w_hash(table, "beta", new_int(2), table->hash_fn("beta"),
	                             JSON_C_OBJECT_ADD_CONSTANT_KEY) == 0);
	assert(lh_table_insert(table, dup_cstr("gamma"), new_int(3)) == 0);

	assert(table->size == 4);
	assert(strcmp((const char *)table->head->k, "alpha") == 0);
	assert(strcmp((const char *)table->head->next->k, "beta") == 0);
	assert(strcmp((const char *)table->tail->k, "gamma") == 0);
	assert(table->tail->prev == table->head->next);

	assert(lh_table_lookup_ex(table, "beta", &found) == 1);
	assert(*(int *)found == 2);

	gamma = lh_table_lookup_entry(table, "gamma");
	assert(gamma != NULL);
	assert(*(int *)gamma->v == 3);

	assert(lh_table_delete(table, "beta") == 0);
	assert(free_count == 1);
	assert(table->count == 2);
	assert(strcmp((const char *)table->head->next->k, "gamma") == 0);

	assert(lh_table_delete_entry(table, gamma) == 0);
	assert(free_count == 2);
	assert(table->count == 1);
	assert(table->head == table->tail);

	lh_table_free(table);
	assert(free_count == 3);
}

static void test_errors_numeric_and_version(void)
{
	double parsed_double = 0.0;
	int64_t parsed_i64 = 0;
	uint64_t parsed_u64 = 0;

	assert(strcmp(json_c_version(), JSON_C_VERSION) == 0);
	assert(json_c_version_num() == JSON_C_VERSION_NUM);
	assert(strcmp(json_number_chars, "0123456789.+-eE") == 0);
	assert(strcmp(json_hex_chars, "0123456789abcdefABCDEF") == 0);

	_json_c_set_last_err("");
	assert(json_util_get_last_err() == NULL);

	_json_c_set_last_err("phase=%d", 2);
	assert(strcmp(json_util_get_last_err(), "phase=2") == 0);

	assert(strcmp(json_type_to_name(json_type_array), "array") == 0);
	assert(json_type_to_name((json_type)99) == NULL);
	assert(strcmp(json_util_get_last_err(),
	              "json_type_to_name: type 99 is out of range [0,7]\n") == 0);

	assert(json_parse_double("12.5", &parsed_double) == 0);
	assert(parsed_double == 12.5);

	errno = 0;
	assert(json_parse_int64("42", &parsed_i64) == 0);
	assert(parsed_i64 == 42);
	assert(errno == 0);

	errno = 0;
	assert(json_parse_int64("x", &parsed_i64) == 1);
	assert(errno == EINVAL);

	errno = 0;
	assert(json_parse_uint64("7", &parsed_u64) == 0);
	assert(parsed_u64 == 7);
	assert(errno == 0);

	errno = 0;
	assert(json_parse_uint64("-1", &parsed_u64) == 1);
	assert(errno == 0);

	errno = 0;
	assert(json_parse_uint64("x", &parsed_u64) == 1);
	assert(errno == EINVAL);
}

static void test_strerror_and_debug(void)
{
	assert(setenv("_JSON_C_STRERROR_ENABLE", "1", 1) == 0);
	assert(strcmp(_json_c_strerror(ENOENT), "ERRNO=ENOENT") == 0);
	assert(strcmp(_json_c_strerror(123456), "ERRNO=123456") == 0);

	assert(mc_get_debug() == 0);
	mc_set_debug(1);
	assert(mc_get_debug() == 1);
	mc_set_syslog(0);
	mc_debug("");
	mc_error("");
	mc_info("");

	(void)json_c_get_random_seed();
}

int main(void)
{
	test_array_list();
	test_printbuf();
	test_hash_modes();
	test_linkhash();
	test_errors_numeric_and_version();
	test_strerror_and_debug();
	puts("foundation_smoke_ok");
	return 0;
}

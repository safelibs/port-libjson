#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "json.h"

static void test_string_serialization(void);
static void test_string_resize_lengths(void);
static void test_binary_string_round_trip(void);
static void test_large_json_output(void);

#ifndef __func__
/* VC++ compat */
#define __func__ __FUNCTION__
#endif

static char *make_filled_string(size_t len, char ch)
{
	char *buf = malloc(len + 1);
	assert(buf != NULL);
	memset(buf, ch, len);
	buf[len] = '\0';
	return buf;
}

static void test_string_serialization(void)
{
	json_object *jso;
	const char *serialized;
	size_t serialized_len;
	char *updated;

	printf("%s: starting test\n", __func__);

	jso = json_object_new_string("blue:1");
	serialized = json_object_to_json_string_length(jso, JSON_C_TO_STRING_PLAIN, &serialized_len);
	assert(strcmp(serialized, "\"blue:1\"") == 0);
	assert(serialized_len == strlen(serialized));
	printf("Serialized string: %s\n", serialized);
	printf("Serialized length: %zu\n", serialized_len);

	updated = malloc(58 + 1);
	assert(updated != NULL);
	memcpy(updated, "blue:1", 6);
	memset(updated + 6, 'x', 52);
	updated[58] = '\0';
	assert(json_object_set_string_len(jso, updated, 58) == 1);

	serialized = json_object_to_json_string_length(jso, JSON_C_TO_STRING_PLAIN, &serialized_len);
	assert(serialized_len == strlen(serialized));
	printf("Updated string: %s\n", serialized);
	printf("Updated serialized length: %zu\n", serialized_len);

	free(updated);
	json_object_put(jso);
	printf("%s: end test\n", __func__);
}

static void test_string_resize_lengths(void)
{
	static const size_t lengths[] = {0, 12, 18, 76, 76, 77};
	size_t ii;
	json_object *jso;

	printf("%s: starting test\n", __func__);
	jso = json_object_new_string("");

	for (ii = 0; ii < sizeof(lengths) / sizeof(lengths[0]); ii++)
	{
		size_t serialized_len;
		const char *serialized;
		char *buf = make_filled_string(lengths[ii], 'x');

		assert(json_object_set_string_len(jso, buf, (int)lengths[ii]) == 1);
		assert((size_t)json_object_get_string_len(jso) == lengths[ii]);

		serialized = json_object_to_json_string_length(jso, JSON_C_TO_STRING_PLAIN,
		                                               &serialized_len);
		assert(serialized_len == strlen(serialized));
		assert(serialized_len == lengths[ii] + 2);
		printf("String length: %zu, serialized length: %zu\n", lengths[ii], serialized_len);

		free(buf);
	}

	json_object_put(jso);
	printf("%s: end test\n", __func__);
}

static void test_binary_string_round_trip(void)
{
	static const char with_nulls[] = {'a', 'b', '\0', 'c'};
	size_t ii;
	size_t serialized_len;
	int reparsed_len;
	const char *serialized;
	const char *reparsed_bytes;
	json_object *jso;
	json_object *reparsed;

	printf("%s: starting test\n", __func__);

	jso = json_object_new_string_len(with_nulls, sizeof(with_nulls));
	serialized = json_object_to_json_string_length(jso, JSON_C_TO_STRING_PLAIN, &serialized_len);
	assert(strcmp(serialized, "\"ab\\u0000c\"") == 0);
	printf("Serialized string: %s\n", serialized);
	printf("Serialized length: %zu\n", serialized_len);

	reparsed = json_tokener_parse(serialized);
	assert(reparsed != NULL);
	reparsed_len = json_object_get_string_len(reparsed);
	reparsed_bytes = json_object_get_string(reparsed);
	assert(reparsed_len == (int)sizeof(with_nulls));
	assert(memcmp(reparsed_bytes, with_nulls, sizeof(with_nulls)) == 0);

	printf("Round-trip length: %d\n", reparsed_len);
	printf("Round-trip bytes:");
	for (ii = 0; ii < (size_t)reparsed_len; ii++)
		printf(" %u", (unsigned char)reparsed_bytes[ii]);
	printf("\n");

	json_object_put(reparsed);
	json_object_put(jso);
	printf("%s: end test\n", __func__);
}

static void test_large_json_output(void)
{
	size_t ii;
	size_t plain_len;
	size_t pretty_len;
	const char *plain;
	const char *pretty;
	char *payload;
	json_object *root;
	json_object *items;

	printf("%s: starting test\n", __func__);

	root = json_object_new_object();
	items = json_object_new_array();
	payload = make_filled_string(64, 'Y');

	for (ii = 0; ii < 40; ii++)
		json_object_array_add(items, json_object_new_int((int)ii));

	json_object_object_add(root, "payload", json_object_new_string_len(payload, 64));
	json_object_object_add(root, "items", items);

	plain = json_object_to_json_string_length(root, JSON_C_TO_STRING_PLAIN, &plain_len);
	assert(plain_len == strlen(plain));
	assert(strstr(plain, "\"payload\":\"") != NULL);
	assert(strstr(plain, "\"items\":[") != NULL);

	pretty = json_object_to_json_string_length(root, JSON_C_TO_STRING_PRETTY, &pretty_len);
	assert(pretty_len == strlen(pretty));

	printf("Plain length: %zu\n", plain_len);
	printf("Pretty length: %zu\n", pretty_len);
	printf("First item: %d\n", json_object_get_int(json_object_array_get_idx(items, 0)));
	printf("Last item: %d\n", json_object_get_int(json_object_array_get_idx(items, 39)));

	free(payload);
	json_object_put(root);
	printf("%s: end test\n", __func__);
}

int main(int argc, char **argv)
{
	(void)argc;
	(void)argv;

	test_string_serialization();
	printf("========================================\n");
	test_string_resize_lengths();
	printf("========================================\n");
	test_binary_string_round_trip();
	printf("========================================\n");
	test_large_json_output();
	printf("========================================\n");

	return 0;
}

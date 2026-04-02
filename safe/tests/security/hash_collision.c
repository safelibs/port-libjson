#ifdef NDEBUG
#undef NDEBUG
#endif

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "json.h"
#include "linkhash.h"

#define COLLISION_HASH_MODULUS 1024UL
#define COLLISION_KEY_COUNT 192

static char *dup_cstr(const char *text)
{
	size_t len = strlen(text) + 1;
	char *copy = malloc(len);

	assert(copy != NULL);
	memcpy(copy, text, len);
	return copy;
}

static unsigned long perllike_hash(const char *text)
{
	unsigned long hashval = 1;

	while (*text != '\0')
		hashval = hashval * 33U + (unsigned char)*text++;
	return hashval;
}

static char **generate_collision_keys(void)
{
	char **keys = calloc(COLLISION_KEY_COUNT, sizeof(*keys));
	char seed_text[32];
	unsigned long target_bucket;
	size_t found = 0;
	unsigned int candidate = 0;

	assert(keys != NULL);

	snprintf(seed_text, sizeof(seed_text), "key-%08x", 0U);
	target_bucket = perllike_hash(seed_text) % COLLISION_HASH_MODULUS;

	while (found < COLLISION_KEY_COUNT)
	{
		char text[32];

		snprintf(text, sizeof(text), "key-%08x", candidate++);
		if ((perllike_hash(text) % COLLISION_HASH_MODULUS) != target_bucket)
			continue;
		keys[found++] = dup_cstr(text);
	}

	return keys;
}

static void free_collision_keys(char **keys)
{
	size_t idx;

	if (keys == NULL)
		return;

	for (idx = 0; idx < COLLISION_KEY_COUNT; idx++)
		free(keys[idx]);
	free(keys);
}

static void append_json_pair(char **buffer, size_t *len, size_t *capacity, const char *key, size_t value,
                             int first)
{
	for (;;)
	{
		size_t remaining = *capacity - *len;
		int written = snprintf(*buffer + *len, remaining, "%s\"%s\":%zu", first ? "" : ",", key, value);

		assert(written >= 0);
		if ((size_t)written < remaining)
		{
			*len += (size_t)written;
			return;
		}

		*capacity *= 2;
		*buffer = realloc(*buffer, *capacity);
		assert(*buffer != NULL);
	}
}

static char *build_collision_json(char **keys)
{
	size_t capacity = COLLISION_KEY_COUNT * 24U;
	size_t len = 0;
	char *buffer = malloc(capacity);
	size_t idx;

	assert(buffer != NULL);
	buffer[len++] = '{';
	buffer[len] = '\0';

	for (idx = 0; idx < COLLISION_KEY_COUNT; idx++)
		append_json_pair(&buffer, &len, &capacity, keys[idx], idx, idx == 0);

	if (len + 2 > capacity)
	{
		capacity += 2;
		buffer = realloc(buffer, capacity);
		assert(buffer != NULL);
	}
	buffer[len++] = '}';
	buffer[len] = '\0';
	return buffer;
}

static struct json_object *parse_object(const char *json_text)
{
	struct json_tokener *tok = json_tokener_new_ex(32);
	struct json_object *obj;
	enum json_tokener_error jerr;

	assert(tok != NULL);
	obj = json_tokener_parse_ex(tok, json_text, (int)strlen(json_text));
	jerr = json_tokener_get_error(tok);
	if (jerr != json_tokener_success)
		fprintf(stderr, "parse error: %s\n", json_tokener_error_desc(jerr));
	assert(jerr == json_tokener_success);
	assert(obj != NULL);
	assert(json_object_is_type(obj, json_type_object));
	json_tokener_free(tok);
	return obj;
}

static void verify_lookup_surface(struct json_object *obj, char **keys)
{
	struct lh_table *table = json_object_get_object(obj);
	size_t idx;

	assert(table != NULL);
	assert(json_object_object_length(obj) == (int)COLLISION_KEY_COUNT);

	for (idx = 0; idx < COLLISION_KEY_COUNT; idx++)
	{
		struct json_object *value = NULL;
		unsigned long hash = lh_get_hash(table, keys[idx]);
		struct lh_entry *entry = lh_table_lookup_entry_w_hash(table, keys[idx], hash);

		assert(json_object_object_get_ex(obj, keys[idx], &value) == 1);
		assert(value != NULL);
		assert(json_object_get_int(value) == (int)idx);
		assert(entry != NULL);
		assert(entry->v == value);
	}
}

static size_t count_unique_buckets(struct lh_table *table, char **keys)
{
	unsigned char *seen = calloc((size_t)table->size, sizeof(*seen));
	size_t unique = 0;
	size_t idx;

	assert(seen != NULL);
	for (idx = 0; idx < COLLISION_KEY_COUNT; idx++)
	{
		size_t bucket = lh_get_hash(table, keys[idx]) % (unsigned long)table->size;

		if (seen[bucket] == 0)
		{
			seen[bucket] = 1;
			unique++;
		}
	}

	free(seen);
	return unique;
}

static void exercise_mode(int hash_mode, const char *json_text, char **keys, size_t min_unique_buckets,
                          size_t max_unique_buckets)
{
	struct json_object *obj;
	struct lh_table *table;
	size_t unique_buckets;

	assert(json_global_set_string_hash(hash_mode) == 0);
	obj = parse_object(json_text);
	table = json_object_get_object(obj);
	assert(table != NULL);
	assert(table->size > 0);
	assert((unsigned long)table->size <= COLLISION_HASH_MODULUS);

	verify_lookup_surface(obj, keys);
	unique_buckets = count_unique_buckets(table, keys);
	assert(unique_buckets >= min_unique_buckets);
	assert(unique_buckets <= max_unique_buckets);

	json_object_put(obj);
}

int main(void)
{
	char **keys = generate_collision_keys();
	char *json_text = build_collision_json(keys);

	exercise_mode(JSON_C_STR_HASH_PERLLIKE, json_text, keys, 1, 1);
	exercise_mode(JSON_C_STR_HASH_DFLT, json_text, keys, COLLISION_KEY_COUNT / 3, COLLISION_KEY_COUNT);
	assert(json_global_set_string_hash(JSON_C_STR_HASH_DFLT) == 0);

	free(json_text);
	free_collision_keys(keys);
	puts("hash_collision_ok");
	return 0;
}

#include <json-c/json.h>

#include <stdio.h>
#include <string.h>

int main(void)
{
	struct json_object *root;
	struct json_object *message;

	root = json_object_new_object();
	if (root == NULL)
		return 1;

	message = json_object_new_string("package-shared-ok");
	if (message == NULL)
	{
		json_object_put(root);
		return 2;
	}

	json_object_object_add(root, "status", message);
	if (!json_object_object_get_ex(root, "status", &message))
	{
		json_object_put(root);
		return 3;
	}

	if (strcmp(json_object_get_string(message), "package-shared-ok") != 0)
	{
		fprintf(stderr, "unexpected json string payload\n");
		json_object_put(root);
		return 4;
	}

	json_object_put(root);
	return 0;
}

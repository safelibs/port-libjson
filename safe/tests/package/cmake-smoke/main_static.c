#include <json-c/arraylist.h>

int main(void)
{
	struct array_list *list;
	char first[] = "alpha";
	char second[] = "beta";

	list = array_list_new(NULL);
	if (list == NULL)
		return 1;

	if (array_list_add(list, first) != 0)
	{
		array_list_free(list);
		return 2;
	}

	if (array_list_insert_idx(list, 1, second) != 0)
	{
		array_list_free(list);
		return 3;
	}

	if (array_list_length(list) != 2)
	{
		array_list_free(list);
		return 4;
	}

	if (array_list_get_idx(list, 0) != first)
	{
		array_list_free(list);
		return 5;
	}

	if (array_list_get_idx(list, 1) != second)
	{
		array_list_free(list);
		return 6;
	}

	array_list_free(list);
	return 0;
}

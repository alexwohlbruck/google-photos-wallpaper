import sys, os 

def resource_path(relative_path):
	if hasattr(sys, '_MEIPASS'):
		return os.path.join(sys._MEIPASS, relative_path)
	return os.path.join(os.path.abspath('.'), relative_path)

# Helper methods
def find_in_list_by_val(list_, key, val):
    return next((d for d in list_ if d[key] == val), None)
    
def find_index_in_list_by_val(list_, key, val):
    return next((i for i, d in enumerate(list_) if d[key] == val), None)

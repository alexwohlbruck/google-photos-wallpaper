import os
import configparser

class Config():

    def __init__(self):
        self.config_path = os.path.abspath('config.ini')
        self.config = configparser.SafeConfigParser()
        self.config.read(self.config_path)

    def get_config(self, section, key):

        if not self.config.has_section(section): return False
        return self.config.get(section, key)

    def set_config(self, section, key, value):
        if not self.config.has_section(section):
            self.config.add_section(section)

        self.config.set(section, key, value)
        self.config.write(open(self.config_path, 'w'))
import threading
import time
import schedule

from src.options import Options

WALL_SCHEDULE = 'WALL_SCHEDULE'

class Scheduler():

	# Start schedule check loop
	def start_loop():
		while True:
			schedule.run_pending()
			time.sleep(1)

	threading.Thread(target = start_loop).start()

    # Restart the schedule on a new interval
	@classmethod
	def start(cls, interval = 10, unit = 'seconds'):
		schedule.clear(WALL_SCHEDULE)
		schedule.every(interval).seconds.do(Options.set_wallpaper_random).tag(WALL_SCHEDULE)
	
    # Stop the schedule
	@classmethod
	def stop(cls):
		schedule.clear(WALL_SCHEDULE)
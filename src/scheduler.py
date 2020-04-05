import threading
import time
import schedule

from src.options import Options

WALL_SCHEDULE = 'WALL_SCHEDULE'
options = Options.get_user_options()

# Start schedule check loop
def start_loop():
	while True:
		schedule.run_pending()
		time.sleep(1)

thread = threading.Thread(target = start_loop)
thread.start()

def test():
	print('job')

class Scheduler():

	@classmethod
	def start(cls, interval = None, unit = 'minutes'):
		"""
		Restart the schedule on a new interval
		Arguments:
		interval (int): The time between wallpaper change
		unit (string)['minutes' | 'hours' | 'days' | 'weeks']: Unit of time used
		"""

		if interval == None:
			settings = options.get('schedule')
			interval = settings.get('interval')
			unit = settings.get('unit')
		
		print('Updating wall every ' + str(interval) + ' ' + unit)

		schedule.clear(WALL_SCHEDULE)

		# TODO: Dynaimcally choose unit to schedule on
		job = getattr(schedule.every(interval), unit)
		job.do(Options.set_wallpaper_random).tag(WALL_SCHEDULE)

		Options.set_schedule(interval, unit)
	
	# Stop the schedule
	@classmethod
	def stop(cls):
		# TODO: Update options.schedule.paused
		schedule.clear(WALL_SCHEDULE)
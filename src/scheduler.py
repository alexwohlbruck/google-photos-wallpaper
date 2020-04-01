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

def job():
	print('job')

class Scheduler():

	# Restart the schedule on a new interval
	@classmethod
	def start(cls, interval = None, unit = 'seconds'):
		if interval == None:
			settings = options.get('schedule')
			interval = settings.get('interval')
			# unit = settings.get('unit')

		schedule.clear(WALL_SCHEDULE)

		print('schedule ' + str(interval))

		# TODO: Dynaimcally choose unit to schedule on
		schedule.every(interval).seconds.do(job).tag(WALL_SCHEDULE)
	
	# Stop the schedule
	@classmethod
	def stop(cls):
		# TODO: Update options.schedule.paused
		schedule.clear(WALL_SCHEDULE)
// SET THE NUM_SLIDERS AND analogInputs TO APPROPRIATE VALUES
// Amount of all slieders
const int NUM_SLIDERS = 4;
// Slider pins - in this order the values will be returned to pc
const int analogInputs[NUM_SLIDERS] = {A0, A1, A2, A3};

void setup() {
  // Set pin 13 (built-in LED)
  pinMode(LED_BUILTIN, OUTPUT);  // Set the LED as an output
  digitalWrite(LED_BUILTIN, HIGH);  // Turn on the LED

  delay(100);

  // Initialize the serial port
  Serial.begin(9600);

  // Wait for the connection to the computer (do not send data until it is ready)
  while (!Serial) {
    // Wait 100 ms and try again
    delay(100);
  }
  digitalWrite(LED_BUILTIN, LOW);

  // Configure the analog pins
  for (int i = 0; i < NUM_SLIDERS; i++) {
    pinMode(analogInputs[i], INPUT);
  }

  // Once the connection is established, turn off the LED
  digitalWrite(LED_BUILTIN, LOW);  // Turn off the LED when the serial port is ready
}

void loop() {
  sendSliderValues();
  delay(100);  // Wait 100 ms before sending the next data
}

void sendSliderValues() {
  String builtString = "";

  // Read slider values and create the appropriate string
  for (int i = 0; i < NUM_SLIDERS; i++) {
    builtString += String(100 - round((analogRead(analogInputs[i]) / 1023.0 ) * 100)); // Inverted values converted to percentages
    if (i < NUM_SLIDERS - 1) {
      builtString += "|";  // Add a separator
    }
  }

  // Send the string through the serial port, but only if the port is ready
  if (Serial) {
    Serial.println(builtString);
  }
}

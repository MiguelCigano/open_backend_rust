
// Mainly: Whats is the colors state on the traffic lights now?
async function fetchTrafficData() {
    try {
        const response = await fetch("/get_value");
        const data = await response.json(); 
        // Wait for: { s1: "red", s2: "green", btn_state: "OK" }
        
        updateLights(1, data.s1);
        updateLights(2, data.s2);
        
        const btnStatus = document.querySelector("#arduino-btn-status span");
        btnStatus.textContent = data.btn_state;
        btnStatus.style.color = data.btn_state === "STOP" ? "#ff0000" : "#00ff6a";

    } 
    catch (err) {
        console.error("[  FAIL] : Error in get data:", err);
    }
}

function updateLights(id, color) {
    // Turn off al lights of this traffic light
    document.querySelectorAll(`#semaforo${id} .light`).forEach(l => l.classList.remove("active"));
    
    // Turn on the correct lights
    const activeLight = document.getElementById(`s${id}-${color}`);
    if (activeLight) activeLight.classList.add("active");
}

async function sendEmergencyStop() {
    await fetch("/set_emergency", { // Reuse the path or create a new
        method: "POST",
        headers: {
            'Content-Type': 'application/json'},
            body: JSON.stringify({ stop: true })
    });
    alert("Emergency signal sent to Backend");
}

setInterval(fetchTrafficData, 300);
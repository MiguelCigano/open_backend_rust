async function fetchTrafficData() {
    try {
        const response = await fetch("/get_value");
        const data = await response.json(); 
        // Esperamos: { s1: "red", s2: "green", btn_fisico: "OK" }
        
        updateLights(1, data.s1);
        updateLights(2, data.s2);
        
        const btnStatus = document.querySelector("#arduino-btn-status span");
        btnStatus.textContent = data.btn_fisico;
        btnStatus.style.color = data.btn_fisico === "PARO" ? "#ff4757" : "#2ed573";

    } 
    catch (err) {
        console.error("Error al obtener datos:", err);
    }
}

function updateLights(id, color) {
    // Apagar todas las luces de ese semáforo
    document.querySelectorAll(`#semaforo${id} .light`).forEach(l => l.classList.remove("active"));
    
    // Encender la correspondiente
    const activeLight = document.getElementById(`s${id}-${color}`);
    if (activeLight) activeLight.classList.add("active");
}

async function sendEmergencyStop() {
    await fetch("/set_emergency", { // Reutilizamos tu ruta o creamos una nueva
        method: "POST",
        headers: {'Content-Type': 'application/x-www-form-urlencoded'},
        body: "red=255&green=0&blue=0" // Puedes mapear esto a una señal de STOP
    });
    alert("Señal de emergencia enviada al backend");
}

setInterval(fetchTrafficData, 300);
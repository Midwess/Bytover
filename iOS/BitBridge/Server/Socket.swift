//
//  Socket.swift
//  BitBridge
//
//  Created by Tien Dang on 3/11/25.
//

import Network
import Foundation
import SwiftUI

class WebSocketServer: ObservableObject {
    private var listener: NWListener?
    private var connectedClients: [NWConnection] = []
    private let queue = DispatchQueue(label: "websocket.server.queue")
    private let port: UInt16
    
    // Callbacks
    var onClientConnected: ((String) -> Void)?
    var onClientDisconnected: ((String) -> Void)?
    var onMessageReceived: ((String, NWConnection) -> Void)?
    var onError: ((Error) -> Void)?
    
    init(port: UInt16 = 8989) {
        self.port = port
    }
    
    func start() {
        do {
            // Create WebSocket parameters
            let parameters = NWParameters.tcp
            parameters.allowLocalEndpointReuse = true
            parameters.includePeerToPeer = true
            
            // Add TLS setup if needed
            // setupTLS(parameters)
            
            // Create the listener
            listener = try NWListener(using: parameters, on: NWEndpoint.Port(integerLiteral: port))
            
            // Set queue and handlers
            listener?.stateUpdateHandler = { [weak self] state in
                self?.handleListenerState(state)
            }
            
            listener?.newConnectionHandler = { [weak self] connection in
                self?.handleNewConnection(connection)
            }
            
            // Start listening
            print("Server starting")
            listener?.start(queue: queue)
            print("Server started")
            
            // Start Bonjour advertising
            advertiseService()
            
        } catch {
            onError?(error)
        }
    }
    
    private func advertiseService() {
        listener?.service = NWListener.Service(name: "iOS-WebSocket-\(UUID().uuidString)",
                                            type: "_websocket._tcp")
    }
    
    private func handleListenerState(_ state: NWListener.State) {
        switch state {
        case .ready:
            if let port = listener?.port {
                print("WebSocket Server running on port \(port)")
            }
        case .failed(let error):
            print("Server failed with error: \(error)")
            onError?(error)
            restart()
        case .cancelled:
            connectedClients.forEach { $0.cancel() }
            connectedClients.removeAll()
        default:
            break
        }
    }
    
    private func handleNewConnection(_ connection: NWConnection) {
        // Set connection parameters
        connection.parameters.allowLocalEndpointReuse = true
        connection.parameters.includePeerToPeer = true
        
        // Handle connection state
        connection.stateUpdateHandler = { [weak self] state in
            self?.handleConnectionState(connection, state: state)
        }
        
        // Start receiving messages
        receiveMessage(from: connection)
        
        // Start the connection
        connection.start(queue: queue)
        connectedClients.append(connection)
    }
    
    private func handleConnectionState(_ connection: NWConnection, state: NWConnection.State) {
        switch state {
        case .ready:
            let clientId = connection.endpoint.debugDescription
            onClientConnected?(clientId)
            
        case .failed(let error):
            print("Connection failed: \(error)")
            removeConnection(connection)
            
        case .cancelled:
            removeConnection(connection)
            
        default:
            break
        }
    }
    
    private func removeConnection(_ connection: NWConnection) {
        if let index = connectedClients.firstIndex(where: { $0 === connection }) {
            let clientId = connection.endpoint.debugDescription
            connectedClients.remove(at: index)
            onClientDisconnected?(clientId)
        }
    }
    
    private func receiveMessage(from connection: NWConnection) {
        // Handle WebSocket frame
        connection.receiveMessage { [weak self] (data, context, isComplete, error) in
            if let error = error {
                print("Receive error: \(error)")
                return
            }
            
            if let data = data, !data.isEmpty {
                // Parse WebSocket frame
                self?.handleWebSocketFrame(data, connection: connection)
            }
            
            if isComplete {
                // Continue receiving messages
                self?.receiveMessage(from: connection)
            }
        }
    }
    
    private func handleWebSocketFrame(_ frameData: Data, connection: NWConnection) {
        // Basic WebSocket frame parsing
        var data = frameData
        guard data.count >= 2 else { return }
        
        let firstByte = data.removeFirst()
        let secondByte = data.removeFirst()
        
        let isFinalFrame = (firstByte & 0x80) != 0
        let opcode = firstByte & 0x0F
        let isMasked = (secondByte & 0x80) != 0
        var payloadLength = UInt64(secondByte & 0x7F)
        
        // Handle different payload lengths
        if payloadLength == 126 {
            guard data.count >= 2 else { return }
            payloadLength = UInt64(data.prefix(2).withUnsafeBytes { $0.load(as: UInt16.self) }.bigEndian)
            data.removeFirst(2)
        } else if payloadLength == 127 {
            guard data.count >= 8 else { return }
            payloadLength = data.prefix(8).withUnsafeBytes { $0.load(as: UInt64.self) }.bigEndian
            data.removeFirst(8)
        }
        
        // Handle masking
        if isMasked {
            guard data.count >= 4 else { return }
            let maskingKey = Array(data.prefix(4))
            data.removeFirst(4)
            
            // Unmask the payload
            for i in 0..<data.count {
                data[i] ^= maskingKey[i % 4]
            }
        }
        
        // Handle the message based on opcode
        switch opcode {
        case 0x1: // Text frame
            if let message = String(data: data, encoding: .utf8) {
                queue.async { [weak self] in
                    self?.onMessageReceived?(message, connection)
                }
            }
            
        case 0x8: // Close frame
            connection.cancel()
            
        case 0x9: // Ping frame
            sendPong(to: connection)
            
        default:
            break
        }
    }
    
    func sendMessage(_ message: String, to connection: NWConnection) {
        // Create WebSocket frame
        var frameData = Data()
        
        // Add first byte (FIN + Opcode)
        frameData.append(0x81) // 1000 0001: Final frame + Text frame
        
        // Add length
        let messageData = message.data(using: .utf8) ?? Data()
        if messageData.count < 126 {
            frameData.append(UInt8(messageData.count))
        } else if messageData.count < 65536 {
            frameData.append(126)
            frameData.append(UInt8(messageData.count >> 8 & 0xFF))
            frameData.append(UInt8(messageData.count & 0xFF))
        } else {
            frameData.append(127)
            frameData.append(UInt8(messageData.count >> 56 & 0xFF))
            frameData.append(UInt8(messageData.count >> 48 & 0xFF))
            frameData.append(UInt8(messageData.count >> 40 & 0xFF))
            frameData.append(UInt8(messageData.count >> 32 & 0xFF))
            frameData.append(UInt8(messageData.count >> 24 & 0xFF))
            frameData.append(UInt8(messageData.count >> 16 & 0xFF))
            frameData.append(UInt8(messageData.count >> 8 & 0xFF))
            frameData.append(UInt8(messageData.count & 0xFF))
        }
        
        // Add payload
        frameData.append(messageData)
        
        // Send the frame
        connection.send(content: frameData, completion: .contentProcessed { [weak self] error in
            if let error = error {
                self?.onError?(error)
            }
        })
    }
    
    private func sendPong(to connection: NWConnection) {
        let pongFrame = Data([0x8A, 0x00]) // Pong frame with no payload
        connection.send(content: pongFrame, completion: .contentProcessed { _ in })
    }
    
    func broadcast(_ message: String) {
        connectedClients.forEach { sendMessage(message, to: $0) }
    }
    
    private func restart() {
        stop()
        start()
    }
    
    func stop() {
        listener?.cancel()
        connectedClients.forEach { $0.cancel() }
        connectedClients.removeAll()
    }
}

//
//  LogoScene.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 12/2/25.
//

import Foundation
import SwiftUI
import SceneKit
import GLTFSceneKit
import CoreMotion

struct LogoScene: UIViewRepresentable {
    private let logoScale: Float = 0.7
    private let motionManager = CMMotionManager()
    
    class Coordinator: NSObject {
        let maxMotion: Float = 0.3
        var parent: LogoScene
        var scene: SCNScene?
        
        init(_ parent: LogoScene) {
            self.parent = parent
            super.init()
            
            // Start motion updates with callback
            parent.motionManager.deviceMotionUpdateInterval = 1.0 / 20.0
            parent.motionManager.startDeviceMotionUpdates(using: .xArbitraryZVertical, to: .main) { [weak self] motion, error in
                guard let motion = motion,
                      let scene = self?.scene else { return }
                
                // Update scene rotation based on device motion
                scene.rootNode.eulerAngles = SCNVector3(
                    Float(max(-0.1, min(0.1, motion.attitude.pitch * 0.3)) * -1),
                    Float(max(-0.2, min(0.2, motion.attitude.roll * 0.3)) * 1),
                    Float(max(-0.2, min(0.2, motion.attitude.roll * 0.1)) * 1)
                )
            }
        }
    }
    
    func makeCoordinator() -> Coordinator {
        Coordinator(self)
    }
    
    func makeUIView(context: Context) -> SCNView {
        let sceneView = SCNView()
        
        sceneView.backgroundColor = .black.withAlphaComponent(0.0)
        sceneView.allowsCameraControl = false
        
        do {
            let sceneSource = GLTFSceneSource(url: Bundle.main.url(forResource: "lightning", withExtension: "gltf")!, options: nil)
            let scene = try sceneSource.scene()
            context.coordinator.scene = scene
            
            sceneView.scene = scene
            
            let ambientLight = SCNNode()
            ambientLight.light = SCNLight()
            ambientLight.light?.type = .ambient
            ambientLight.light?.intensity = 100
            ambientLight.light?.color = UIColor.init(Theme.LightViolet.color)
            scene.rootNode.addChildNode(ambientLight)
            
            let directionalLight = SCNNode()
            directionalLight.light = SCNLight()
            directionalLight.light?.type = .directional
            directionalLight.light?.intensity = 6000
            directionalLight.light?.color = UIColor.init(Theme.LightViolet.color)
            directionalLight.position = SCNVector3(x: 1, y: 0, z: 2)
            directionalLight.eulerAngles = SCNVector3(x: -0.5, y: 0.5, z: 0)
            scene.rootNode.addChildNode(directionalLight)
            
            scene.rootNode.scale.x = logoScale
            scene.rootNode.scale.y = logoScale
            scene.rootNode.scale.z = logoScale
            
            scene.rootNode.enumerateChildNodes { (node, _) in
                if node.parent?.name == "lightning-border" {
                    if let geometry = node.geometry {
                        let material = SCNMaterial()
                        material.diffuse.contents = UIColor.init(Theme.SecondaryBlue.color)
                        material.metalness.contents = 0.3
                        material.roughness.contents = 0.2
                        material.lightingModel = .physicallyBased
                        
                        geometry.materials = [material]
                    }
                }
                else if node.parent?.name == "lightning-body" {
                    if let geometry = node.geometry {
                        let material = SCNMaterial()
                        material.diffuse.contents = UIColor.init(Theme.PrimaryViolet.color)
                        material.metalness.contents = 1.0
                        material.roughness.contents = 0.3
                        material.lightingModel = .physicallyBased
                        
                        geometry.materials = [material]
                    }
                }
            }
            
        } catch {
            print("Error loading GLTF: \(error.localizedDescription)")
        }
        
        return sceneView
    }
    
    func updateUIView(_ uiView: SCNView, context: Context) {}
    
    static func dismantleUIView(_ uiView: SCNView, coordinator: Coordinator) {
        coordinator.parent.motionManager.stopDeviceMotionUpdates()
    }
}

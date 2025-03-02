//
//  ShareView.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 19/2/25.
//

import SwiftUI
import Foundation
import SharedTypes
import SceneKit

public struct ShareView: View {
    @Environment(\.safeAreaInsets) private var safeAreaInsets
    @Environment(\.screenSize) private var screenSize
    @EnvironmentObject var core: Core
    @State private var startTime = Date.now
    
    public var body: some View {
        ZStack(alignment: .bottom) {
            StunningBackgroundGradient()
            ScrollView(.vertical) {
                VStack(alignment: .center) {
                    LogoScene(gltfFileName: "Rocket", logoScale: 1.7)
                        .frame(width: screenSize.width, height: 100)
                        .overlay(Theme.gradientHeading.opacity(0.5).blur(radius: 15).frame(width: .infinity, height: screenSize.width / 2))
                    
                    Text("Your dashboard")
                        .padding(.horizontal, 20)
                        .multilineTextAlignment(.center)
                        .modifier(Heading2())
                    
                    Spacer().frame(height: 10)
                    
                    UpgradePremiumButton()
                    
                    if core.transfer?.selected_resources.isEmpty ?? true {
                        Spacer().frame(height: 150)
                    }
                    
                    ContentPickerView()
                        .padding(.trailing, SpaceTheme.screen.value - 10)
                    
                    ForEach(core.transfer?.selected_resources ?? [], id: \.self) { item in
                        SelectedResourceItem(resource: item)
                            .padding(.horizontal, 15)
                            .padding(.bottom, 8)
                    }
                    
                    Spacer().frame(width: 10, height: 120)
                }
            }
            .padding(.bottom, 120)
            .padding(.top, safeAreaInsets.top)
            
            Rectangle()
                .foregroundStyle(Theme.darkBgGradient.opacity(0.9))
                .blur(radius: 35)
                .frame(width: .infinity, height: 230)
                .padding(-20)
            
            ShareButton(width: 130)
                .padding(.bottom, 80)
        }
        .onAppear() {
            core.update(.transfer(.initSession))
        }
        .ignoresSafeArea()
    }
}

#Preview {
   ShareView()
        .environmentObject(CoreMock.withSelectedFileTransfers())
}

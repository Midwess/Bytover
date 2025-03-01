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
                    
                    Spacer().frame(height: 150)
                    
                    ContentPickerView()
                        .padding(.horizontal, 19)

                    ForEach(core.transfer?.selected_resources ?? [], id: \.self) { item in
                        SelectedResourceItem(resource: item)
                            .padding(.horizontal, 15)
                            .padding(.top, 10)
                    }
                    Spacer().frame(width: 10, height: 210)
                }
            }
            .padding(.bottom, 100)
            .padding(.top, safeAreaInsets.top)

            ShareButton(width: 150)
                .padding(.bottom, 80)
        }
        .onAppear() {
            print("On appear")
            core.update(.transfer(.initSession))
        }
        .ignoresSafeArea()
    }
}

#Preview {
   ShareView()
        .environmentObject(CoreMock.withSelectedFileTransfers())
}
